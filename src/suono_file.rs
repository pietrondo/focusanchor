//! Audio registrato da file, in alternativa ai generatori procedurali.
//!
//! Il rumore generato in tempo reale copre i pensieri intrusivi, ma a lungo
//! andare risulta fastidioso: un file scelto dall'utente suona naturale e si
//! puo' cambiare senza ricompilare.

use std::fs::File;
use std::path::{Path, PathBuf};

use rodio::{Decoder, Source};

/// Nome del file cercato accanto all'eseguibile.
pub const FILE_PIOGGIA: &str = "pioggia.mp3";

/// Sorgente che rigira un file audio all'infinito.
///
/// `Source::repeat_infinite` bufferizza tutto il file in memoria: nove ore
/// sarebbero gigabytes. Qui il file si riapre da capo a ogni fine.
pub struct SuonoFile {
    percorso: PathBuf,
    decodifica: Decoder<File>,
}

impl SuonoFile {
    /// Apre il file indicato, con un messaggio leggibile se manca o non si
    /// decodifica.
    pub fn apri(percorso: impl AsRef<Path>) -> Result<Self, String> {
        let percorso = percorso.as_ref().to_path_buf();
        let decodifica = apri_decodifica(&percorso)?;
        Ok(Self {
            percorso,
            decodifica,
        })
    }
}

fn apri_decodifica(percorso: &Path) -> Result<Decoder<File>, String> {
    let file = File::open(percorso).map_err(|e| {
        format!(
            "Manca il file audio {}: {e}. Mettilo accanto a focusanchor.exe.",
            percorso.display()
        )
    })?;
    Decoder::new(file)
        .map_err(|e| format!("File audio non leggibile ({}): {e}", percorso.display()))
}

impl Iterator for SuonoFile {
    type Item = i16;

    fn next(&mut self) -> Option<i16> {
        if let Some(campione) = self.decodifica.next() {
            return Some(campione);
        }
        // Fine del file: si riparte da capo. Se la riapertura fallisce il
        // suono si ferma in silenzio, senza far cadere l'app.
        self.decodifica = apri_decodifica(&self.percorso).ok()?;
        self.decodifica.next()
    }
}

impl Source for SuonoFile {
    fn current_frame_len(&self) -> Option<usize> {
        self.decodifica.current_frame_len()
    }

    fn channels(&self) -> u16 {
        self.decodifica.channels()
    }

    fn sample_rate(&self) -> u32 {
        self.decodifica.sample_rate()
    }

    fn total_duration(&self) -> Option<std::time::Duration> {
        None
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn un_file_mancante_spiega_dove_metterlo() {
        match SuonoFile::apri("file_che_non_esiste.m4a") {
            Ok(_) => panic!("un file inesistente non deve aprirsi"),
            Err(errore) => assert!(errore.contains("file_che_non_esiste.m4a"), "{errore}"),
        }
    }

    /// Il file vero si prova qui: un errore di decodifica a runtime sarebbe
    /// solo silenzio. Il test e' `#[ignore]` perche' richiede `pioggia.mp3`
    /// accanto all'eseguibile di release.
    #[test]
    #[ignore]
    fn il_file_scaricato_si_decodifica() {
        let percorso = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("target/release")
            .join(FILE_PIOGGIA);
        let mut suono = SuonoFile::apri(&percorso).expect("file audio");
        let campioni: Vec<i16> = (0..44_100).map(|_| suono.next().unwrap_or(0)).collect();
        assert!(campioni.iter().any(|c| *c != 0), "audio muto");
    }
}
