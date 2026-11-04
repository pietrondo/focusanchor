//! Motore audio: due modalita' procedurali di sottofondo.
//!
//! - `Rumore Marrone`: maschera i pensieri intrusivi.
//! - `Binaurale 40Hz`: batte binaurale che contrasta la sonnolenza.
//!
//! La voce (TTS) e' in `audio_voce.rs`, i sorgenti in `suono.rs`.

use std::process::Child;
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use rodio::{OutputStream, OutputStreamHandle, Sink};

use crate::suono::{Beep, Binaurale40, RumoreMarrone};
use crate::suono_file::{SuonoFile, FILE_PIOGGIA};
use crate::suono_pioggia::Pioggia;

/// Modalita' di audio di sottofondo scelta dall'utente.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ModalitaAudio {
    /// Nessun suono di sottofondo.
    Spenta,
    /// Rumore marrone per mascherare i pensieri intrusivi.
    RumoreMarrone,
    /// Pioggia: stesso mascheramento, ma con un suono piu' naturale.
    Pioggia,
    /// Pioggia registrata, letta dal file accanto all'eseguibile.
    PioggiaFile,
    /// Battito binaurale a 40 Hz per contrastare la sonnolenza.
    Binaurale40,
}

impl ModalitaAudio {
    /// Etichetta da mostrare nella interfaccia.
    pub fn etichetta(&self) -> &'static str {
        match self {
            ModalitaAudio::Spenta => "Spento",
            ModalitaAudio::RumoreMarrone => "Rumore Marrone",
            ModalitaAudio::Pioggia => "Pioggia",
            ModalitaAudio::PioggiaFile => "Pioggia naturale",
            ModalitaAudio::Binaurale40 => "Sveglia 40Hz Anti-Sonno",
        }
    }

    /// Modalita' successiva nel ciclo del selettore.
    pub fn successiva(&self) -> Self {
        match self {
            ModalitaAudio::Spenta => ModalitaAudio::RumoreMarrone,
            ModalitaAudio::RumoreMarrone => ModalitaAudio::Pioggia,
            ModalitaAudio::Pioggia => ModalitaAudio::PioggiaFile,
            ModalitaAudio::PioggiaFile => ModalitaAudio::Binaurale40,
            ModalitaAudio::Binaurale40 => ModalitaAudio::Spenta,
        }
    }

    /// Modalita' precedente nel ciclo del selettore.
    pub fn precedente(&self) -> Self {
        match self {
            ModalitaAudio::Spenta => ModalitaAudio::Binaurale40,
            ModalitaAudio::Pioggia => ModalitaAudio::RumoreMarrone,
            ModalitaAudio::PioggiaFile => ModalitaAudio::Pioggia,
            ModalitaAudio::RumoreMarrone => ModalitaAudio::Spenta,
            ModalitaAudio::Binaurale40 => ModalitaAudio::RumoreMarrone,
        }
    }

    /// Nome salvato in configurazione.
    ///
    /// La conversione sta qui e non nello stato: chi aggiunge una modalita'
    /// non deve ricordarsi di cercare le due match sparse in altri file.
    pub fn nome(&self) -> &'static str {
        match self {
            ModalitaAudio::Spenta => "Spenta",
            ModalitaAudio::RumoreMarrone => "RumoreMarrone",
            ModalitaAudio::Pioggia => "Pioggia",
            ModalitaAudio::PioggiaFile => "PioggiaFile",
            ModalitaAudio::Binaurale40 => "Binaurale40",
        }
    }

    /// Modalita' letta dalla configurazione, con il marrone come ripiego.
    pub fn da_nome(nome: &str) -> Self {
        match nome {
            "Binaurale40" => ModalitaAudio::Binaurale40,
            "Spenta" => ModalitaAudio::Spenta,
            "Pioggia" => ModalitaAudio::Pioggia,
            "PioggiaFile" => ModalitaAudio::PioggiaFile,
            _ => ModalitaAudio::RumoreMarrone,
        }
    }

    /// Vero se la modalita' produce suono.
    pub fn attiva(&self) -> bool {
        !matches!(self, ModalitaAudio::Spenta)
    }
}

/// Messaggio di stato della voce.
#[derive(Debug, Clone)]
pub enum Voce {
    /// La lettura e' iniziata.
    Iniziata,
    /// La lettura e' terminata.
    Terminata,
    /// La lettura e' stata interrotta dall'utente.
    Interrotta,
    /// Errore durante la lettura.
    Errore(String),
}

/// Motore audio: gestisce il sottofondo e la voce.
pub struct MotoreAudio {
    /// Stream di uscita, tenuto vivo finche' il motore esiste.
    _stream: Option<OutputStream>,
    /// Handle dello stream, usato per creare i sink.
    handle: Option<OutputStreamHandle>,
    /// Sink del suono di sottofondo.
    sink_sottofondo: Option<Sink>,
    /// Volume scelto dall'utente per il sottofondo.
    ///
    /// Va ricordato: mentre parla la voce il sottofondo viene abbassato, e
    /// senza questo valore non si saprebbe a quanto rimetterlo.
    volume_sottofondo: f32,
    /// Modalita' attualmente in riproduzione.
    modalita: ModalitaAudio,
    /// Vero se la voce e' abilitata.
    pub voce_attiva: bool,
    /// Sender per i messaggi di stato della voce.
    pub(crate) tx: Sender<Voce>,
    /// Receiver per i messaggi di stato della voce.
    pub(crate) rx: Receiver<Voce>,
    /// Processi PowerShell attivi (per poterli interrompere).
    pub(crate) processi: Arc<Mutex<Vec<Child>>>,
}

impl MotoreAudio {
    /// Crea il motore audio (senza avviare subito l'output).
    pub fn nuovo() -> Self {
        let (tx, rx) = mpsc::channel();
        Self {
            _stream: None,
            handle: None,
            sink_sottofondo: None,
            volume_sottofondo: 0.35,
            modalita: ModalitaAudio::Spenta,
            voce_attiva: true,
            tx,
            rx,
            processi: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Avvia (o riavvia) lo stream audio di uscita.
    fn assicura_stream(&mut self) -> bool {
        if self.handle.is_some() {
            return true;
        }
        match OutputStream::try_default() {
            Ok((stream, handle)) => {
                self._stream = Some(stream);
                self.handle = Some(handle);
                true
            }
            Err(_) => false,
        }
    }

    /// Accende o spegne il suono di sottofondo nella modalita' scelta.
    pub fn imposta_modalita(
        &mut self,
        modalita: ModalitaAudio,
        volume: f32,
    ) -> Result<(), String> {
        if !modalita.attiva() {
            self.ferma_sottofondo();
            return Ok(());
        }
        if self.modalita == modalita && self.sink_sottofondo.is_some() {
            self.imposta_volume(volume);
            return Ok(());
        }
        self.ferma_sottofondo();
        if !self.assicura_stream() {
            return Err("Impossibile aprire il dispositivo audio di uscita.".into());
        }
        let handle = self.handle.as_ref().expect("stream appena creato");
        let sink = Sink::try_new(handle).map_err(|e| e.to_string())?;
        self.volume_sottofondo = volume.clamp(0.0, 1.0);
        sink.set_volume(self.volume_sottofondo);
        match modalita {
            ModalitaAudio::Binaurale40 => sink.append(Binaurale40::nuovo()),
            ModalitaAudio::Pioggia => sink.append(Pioggia::nuovo()),
            ModalitaAudio::PioggiaFile => sink.append(SuonoFile::apri(
                crate::config::cartella_dati().join(FILE_PIOGGIA),
            )?),
            _ => sink.append(RumoreMarrone::nuovo()),
        }
        self.sink_sottofondo = Some(sink);
        self.modalita = modalita;
        Ok(())
    }

    /// Regola il volume del suono di sottofondo.
    pub fn imposta_volume(&mut self, volume: f32) {
        self.volume_sottofondo = volume.clamp(0.0, 1.0);
        if let Some(sink) = &self.sink_sottofondo {
            sink.set_volume(self.volume_sottofondo);
        }
    }

    /// Abbassa il sottofondo mentre parla la voce.
    ///
    /// La voce non e' una traccia del nostro mixer: e' Windows che parla da
    /// un altro processo. Senza abbassare il rumore, la voce resta sotto e
    /// non si capisce niente.
    pub fn abbassa_sottofondo(&mut self) {
        if let Some(sink) = &self.sink_sottofondo {
            sink.set_volume(volume_abbassato(self.volume_sottofondo));
        }
    }

    /// Riporta il sottofondo al volume scelto.
    pub fn ripristina_sottofondo(&mut self) {
        if let Some(sink) = &self.sink_sottofondo {
            sink.set_volume(self.volume_sottofondo);
        }
    }

    /// Spegne il suono di sottofondo.
    pub fn ferma_sottofondo(&mut self) {
        if let Some(sink) = self.sink_sottofondo.take() {
            sink.stop();
        }
        self.modalita = ModalitaAudio::Spenta;
    }

    /// Emette un breve beep di sveglia.
    pub fn beep(&mut self) {
        if !self.assicura_stream() {
            return;
        }
        let Some(handle) = self.handle.as_ref() else {
            return;
        };
        let Ok(sink) = Sink::try_new(handle) else {
            return;
        };
        sink.set_volume(0.5);
        sink.append(Beep::nuovo(Duration::from_millis(180), 880.0));
        sink.append(Beep::nuovo(Duration::from_millis(180), 880.0));
    }

    /// Preleva l'ultimo messaggio di stato della voce, se presente.
    pub fn stato_voce(&mut self) -> Option<Voce> {
        self.pulizia_processi();
        self.rx.try_recv().ok()
    }
}

/// Volume del sottofondo mentre parla la voce.
///
/// Un quinto: la voce arriva da un altro programma e non passa dal nostro
/// mixer, quindi deve avere la strada libera.
fn volume_abbassato(volume: f32) -> f32 {
    (volume * 0.2).clamp(0.0, 1.0)
}

impl Drop for MotoreAudio {
    fn drop(&mut self) {
        self.ferma_voce();
        self.ferma_sottofondo();
    }
}

#[cfg(test)]
mod test {
    use super::*;

    /// La modalita' scelta si salva e si rilegge in configurazione.
    ///
    /// Senza questo, "Pioggia naturale" tornerebbe "Rumore marrone" a ogni
    /// riavvio e il file scaricato non si sentirebbe mai.
    #[test]
    fn la_pioggia_da_file_sopravvive_al_salvataggio() {
        let modalita = ModalitaAudio::PioggiaFile;
        assert_eq!(ModalitaAudio::da_nome(modalita.nome()), modalita);
        assert!(modalita.attiva());
    }

    #[test]
    fn il_selettore_arriva_alla_pioggia_da_file() {
        assert_eq!(
            ModalitaAudio::Pioggia.successiva(),
            ModalitaAudio::PioggiaFile
        );
        assert_eq!(
            ModalitaAudio::PioggiaFile.precedente(),
            ModalitaAudio::Pioggia
        );
    }
}
