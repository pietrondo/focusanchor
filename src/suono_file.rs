//! Audio registrato da file, in alternativa ai generatori procedurali.
//!
//! Il rumore generato in tempo reale copre i pensieri intrusivi, ma a lungo
//! andare risulta fastidioso: un file scelto dall'utente suona naturale e si
//! puo' cambiare senza ricompilare.

use std::collections::VecDeque;
use std::fs::File;
use std::io::Cursor;
use std::path::{Path, PathBuf};

use rodio::{Decoder, Source};

/// Nome del file cercato accanto all'eseguibile.
pub const FILE_PIOGGIA: &str = "pioggia.mp3";

/// Percorso del file audio letto come pioggia.
///
/// Prima accanto all'eseguibile, dove lo mette il README; se non c'e' la
/// cartella dati, cosi' chi lo sposta li' lo trova lo stesso.
pub fn percorso_file() -> PathBuf {
    let vicino = crate::config::cartella_programma().join(FILE_PIOGGIA);
    if vicino.exists() {
        return vicino;
    }
    crate::config::cartella_dati().join(FILE_PIOGGIA)
}

/// Durata della sovrapposizione fra la fine e l'inizio della traccia.
const DURATA_SFUMATURA: usize = 80;

/// Sorgente che rigira un file audio all'infinito.
///
/// `Source::repeat_infinite` bufferizza tutto il file in memoria: nove ore
/// sarebbero gigabytes. Qui il file si riapre da capo a ogni fine.
pub struct SuonoFile {
    ciclo: CicloAudio<Box<dyn Iterator<Item = i16> + Send>>,
}

struct CicloAudio<S> {
    decodifica: S,
    riapri: Box<dyn FnMut() -> Option<S> + Send>,
    canali: u16,
    frequenza: u32,
    campioni_sfumatura: usize,
    campioni_letti: usize,
    coda: VecDeque<i16>,
    uscita: VecDeque<i16>,
    terminato: bool,
}

impl SuonoFile {
    /// Apre il file indicato, con un messaggio leggibile se manca o non si
    /// decodifica.
    pub fn apri(percorso: impl AsRef<Path>) -> Result<Self, String> {
        let percorso = percorso.as_ref().to_path_buf();
        let decodifica = apri_decodifica(&percorso)?;
        let canali = decodifica.channels();
        let frequenza = decodifica.sample_rate();
        let decodifica: Box<dyn Iterator<Item = i16> + Send> = Box::new(decodifica);
        let percorso_riapertura = percorso.clone();
        let ciclo = CicloAudio::nuovo(decodifica, canali, frequenza, move || {
            Some(Box::new(apri_decodifica(&percorso_riapertura).ok()?)
                as Box<dyn Iterator<Item = i16> + Send>)
        });
        Ok(Self { ciclo })
    }

    /// Decodifica e cicla un MP3 incorporato nell'eseguibile.
    pub fn incorporato(bytes: &'static [u8]) -> Result<Self, String> {
        let decodifica = Decoder::new(Cursor::new(bytes))
            .map_err(|e| format!("Audio incorporato non leggibile: {e}"))?;
        let canali = decodifica.channels();
        let frequenza = decodifica.sample_rate();
        let decodifica: Box<dyn Iterator<Item = i16> + Send> = Box::new(decodifica);
        let ciclo = CicloAudio::nuovo(decodifica, canali, frequenza, move || {
            Some(Box::new(Decoder::new(Cursor::new(bytes)).ok()?)
                as Box<dyn Iterator<Item = i16> + Send>)
        });
        Ok(Self { ciclo })
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
        self.ciclo.next()
    }
}

impl<S: Iterator<Item = i16>> CicloAudio<S> {
    fn nuovo<F>(decodifica: S, canali: u16, frequenza: u32, riapri: F) -> Self
    where
        F: FnMut() -> Option<S> + Send + 'static,
    {
        let canali = canali.max(1);
        let campioni_sfumatura = ((frequenza as usize * canali as usize * DURATA_SFUMATURA)
            / 1_000)
            .max(canali as usize);
        let campioni_sfumatura = campioni_sfumatura - campioni_sfumatura % canali as usize;
        Self {
            decodifica,
            riapri: Box::new(riapri),
            canali,
            frequenza,
            campioni_sfumatura,
            campioni_letti: 0,
            coda: VecDeque::with_capacity(campioni_sfumatura + 1),
            uscita: VecDeque::with_capacity(campioni_sfumatura),
            terminato: false,
        }
    }
}

impl<S: Iterator<Item = i16>> Iterator for CicloAudio<S> {
    type Item = i16;

    fn next(&mut self) -> Option<i16> {
        loop {
            if let Some(campione) = self.uscita.pop_front() {
                return Some(campione);
            }
            if self.terminato {
                return self.coda.pop_front();
            }

            if let Some(campione) = self.decodifica.next() {
                self.campioni_letti += 1;
                self.coda.push_back(campione);
                if self.coda.len() > self.campioni_sfumatura {
                    return self.coda.pop_front();
                }
                continue;
            }

            if self.coda.is_empty() {
                self.terminato = true;
                return None;
            }

            let coda: Vec<i16> = self.coda.drain(..).collect();
            let breve = self.campioni_letti < self.campioni_sfumatura;
            let overlap = if breve {
                (coda.len() / self.canali as usize / 2) * self.canali as usize
            } else {
                self.campioni_sfumatura.min(coda.len())
            };
            self.decodifica = match (self.riapri)() {
                Some(decodifica) => decodifica,
                None => {
                    self.coda.extend(coda);
                    self.terminato = true;
                    continue;
                }
            };
            self.campioni_letti = 0;

            if overlap == 0 {
                self.uscita.extend(coda);
                continue;
            }

            let mut inizio = Vec::with_capacity(overlap);
            for _ in 0..overlap {
                let Some(campione) = self.decodifica.next() else {
                    break;
                };
                inizio.push(campione);
            }
            if inizio.is_empty() {
                self.coda.extend(coda);
                self.terminato = true;
                continue;
            }
            self.campioni_letti = inizio.len();

            let canali = self.canali as usize;
            inizio.truncate(inizio.len() - inizio.len() % canali);
            if inizio.is_empty() {
                self.coda.extend(coda);
                self.terminato = true;
                continue;
            }

            let prefisso = coda.len().saturating_sub(inizio.len());
            self.uscita.extend(coda[..prefisso].iter().copied());
            self.uscita
                .extend(crossfade_samples(&coda[prefisso..], &inizio, self.canali));
        }
    }
}

fn crossfade_samples(coda: &[i16], inizio: &[i16], canali: u16) -> Vec<i16> {
    let canali = canali.max(1) as usize;
    let lunghezza = coda.len().min(inizio.len());
    let lunghezza = lunghezza - lunghezza % canali;
    let frame = lunghezza / canali;
    (0..lunghezza)
        .map(|indice| {
            let peso = if frame <= 1 {
                1.0
            } else {
                (indice / canali) as f32 / (frame - 1) as f32
            };
            let campione = coda[indice] as f32 * (1.0 - peso) + inizio[indice] as f32 * peso;
            campione.round().clamp(i16::MIN as f32, i16::MAX as f32) as i16
        })
        .collect()
}

impl Source for SuonoFile {
    fn current_frame_len(&self) -> Option<usize> {
        None
    }

    fn channels(&self) -> u16 {
        self.ciclo.canali
    }

    fn sample_rate(&self) -> u32 {
        self.ciclo.frequenza
    }

    fn total_duration(&self) -> Option<std::time::Duration> {
        None
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    fn traccia_stereo(frame: usize) -> Vec<i16> {
        (0..frame)
            .flat_map(|indice| [indice as i16, 10_000 + indice as i16])
            .collect()
    }

    #[test]
    fn il_ciclo_attraversa_due_ripartenze() {
        let traccia = traccia_stereo(200);
        let riaperture = Arc::new(AtomicUsize::new(0));
        let conteggio = Arc::clone(&riaperture);
        let traccia_da_riaprire = traccia.clone();
        let mut ciclo = CicloAudio::nuovo(traccia.into_iter(), 2, 1_000, move || {
            conteggio.fetch_add(1, Ordering::Relaxed);
            Some(traccia_da_riaprire.clone().into_iter())
        });

        let campioni: Vec<i16> = (0..642).map(|_| ciclo.next().unwrap()).collect();

        assert_eq!(riaperture.load(Ordering::Relaxed), 2);
        assert_eq!(&campioni[..240], &traccia_stereo(200)[..240]);
    }

    #[test]
    fn la_traccia_più_corta_della_sfumatura_continua_a_ciclare() {
        let traccia = traccia_stereo(3);
        let riaperture = Arc::new(AtomicUsize::new(0));
        let conteggio = Arc::clone(&riaperture);
        let traccia_da_riaprire = traccia.clone();
        let mut ciclo = CicloAudio::nuovo(traccia.into_iter(), 2, 1_000, move || {
            conteggio.fetch_add(1, Ordering::Relaxed);
            Some(traccia_da_riaprire.clone().into_iter())
        });

        let campioni: Vec<i16> = (0..24).map(|_| ciclo.next().unwrap()).collect();

        assert!(riaperture.load(Ordering::Relaxed) >= 3);
        assert!(campioni
            .chunks_exact(2)
            .all(|frame| frame[1] - frame[0] == 10_000));
    }

    #[test]
    fn il_ciclo_preserva_allineamento_e_ordine_fuori_dalla_sfumatura() {
        let traccia = traccia_stereo(200);
        let traccia_da_riaprire = traccia.clone();
        let mut ciclo = CicloAudio::nuovo(traccia.into_iter(), 2, 1_000, move || {
            Some(traccia_da_riaprire.clone().into_iter())
        });

        let campioni: Vec<i16> = (0..402).map(|_| ciclo.next().unwrap()).collect();

        assert_eq!(&campioni[..240], &traccia_stereo(200)[..240]);
        assert_eq!(&campioni[400..402], &traccia_stereo(200)[160..162]);
        assert!(campioni
            .chunks_exact(2)
            .all(|frame| frame[1] - frame[0] == 10_000));
    }

    #[test]
    fn una_riapertura_fallita_emette_la_coda_e_termina() {
        let traccia = traccia_stereo(3);
        let mut ciclo = CicloAudio::nuovo(traccia.clone().into_iter(), 2, 1_000, || None);

        let campioni: Vec<i16> = ciclo.by_ref().collect();

        assert_eq!(campioni, traccia);
        assert_eq!(ciclo.next(), None);
    }

    #[test]
    fn il_loop_sfuma_senza_salti_bruschi() {
        let tail = vec![24_000; 128];
        let head = vec![-24_000; 128];

        let sfumatura = crossfade_samples(&tail, &head, 2);

        assert_eq!(sfumatura.len(), tail.len());
        assert_eq!(sfumatura[0], tail[0]);
        assert_eq!(*sfumatura.last().unwrap(), *head.last().unwrap());
        let salto_massimo = sfumatura
            .windows(2)
            .map(|coppia| (coppia[1] as i32 - coppia[0] as i32).abs())
            .max()
            .unwrap();
        assert!(
            salto_massimo < 1_000,
            "la sfumatura lascia un salto di {salto_massimo}"
        );
        assert!(sfumatura.chunks_exact(2).all(|frame| frame[0] == frame[1]));
    }

    #[test]
    fn un_file_mancante_spiega_dove_metterlo() {
        match SuonoFile::apri("file_che_non_esiste.m4a") {
            Ok(_) => panic!("un file inesistente non deve aprirsi"),
            Err(errore) => assert!(errore.contains("file_che_non_esiste.m4a"), "{errore}"),
        }
    }

    #[test]
    fn decodifica_i_due_suoni_rain_bundled() {
        let bytes_totali = include_bytes!("../assets/audio/rain-city.mp3").len()
            + include_bytes!("../assets/audio/gentle-rain.mp3").len();
        assert!(bytes_totali <= 10 * 1024 * 1024);
        for (bytes, sample_rate) in [
            (
                include_bytes!("../assets/audio/rain-city.mp3").as_slice(),
                44_100,
            ),
            (
                include_bytes!("../assets/audio/gentle-rain.mp3").as_slice(),
                24_000,
            ),
        ] {
            let source = SuonoFile::incorporato(bytes).expect("MP3 bundled decodificabile");
            assert_eq!(source.channels(), 2);
            assert_eq!(source.sample_rate(), sample_rate);
            assert!(source.take(4096).any(|sample| sample != 0));
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
