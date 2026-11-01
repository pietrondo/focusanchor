//! Text-to-Speech nativo di Windows e lettura del testo copiato.
//!
//! La voce usa `System.Speech.Synthesis.SpeechSynthesizer` di Windows
//! PowerShell 5.1: nessuna libreria C++ da compilare, quindi `cargo build`
//! non puo' fallire per dipendenze mancanti.

use std::io::Write;
use std::os::windows::process::CommandExt;
use std::process::{Command, Stdio};

use crate::audio::{MotoreAudio, Voce};
use crate::blocker::CREATE_NO_WINDOW;
use crate::tts::script_tts;

/// Velocita' di lettura della voce, pensata per l'ADHD.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VelocitaVoce {
    /// Velocita' normale (Rate = 1).
    Normale,
    /// Velocita' sostenuta (Rate = 3).
    Veloce,
    /// Velocita' massima (Rate = 5).
    Turbo,
}

impl VelocitaVoce {
    /// Etichetta da mostrare nella interfaccia.
    pub fn etichetta(&self) -> &'static str {
        match self {
            VelocitaVoce::Normale => "Normale",
            VelocitaVoce::Veloce => "Veloce",
            VelocitaVoce::Turbo => "Turbo",
        }
    }

    /// Valore `Rate` corrispondente per `SpeechSynthesizer`.
    pub fn rate(&self) -> i32 {
        match self {
            VelocitaVoce::Normale => 1,
            VelocitaVoce::Veloce => 3,
            VelocitaVoce::Turbo => 5,
        }
    }

    /// Modalita' successiva nel ciclo del selettore.
    pub fn successiva(&self) -> Self {
        match self {
            VelocitaVoce::Normale => VelocitaVoce::Veloce,
            VelocitaVoce::Veloce => VelocitaVoce::Turbo,
            VelocitaVoce::Turbo => VelocitaVoce::Normale,
        }
    }

    /// Modalita' precedente nel ciclo del selettore.
    pub fn precedente(&self) -> Self {
        match self {
            VelocitaVoce::Normale => VelocitaVoce::Turbo,
            VelocitaVoce::Veloce => VelocitaVoce::Normale,
            VelocitaVoce::Turbo => VelocitaVoce::Veloce,
        }
    }
}

impl MotoreAudio {
    /// Legge il testo copiato negli appunti e lo restituisce ripulito.
    ///
    /// Il rumore di fondo della tastiera e i salti di riga spezzati dei PDF
    /// vengono ripuliti per non far inciampare la voce.
    pub fn leggi_clipboard() -> Result<String, String> {
        let mut clipboard = arboard::Clipboard::new().map_err(|e| e.to_string())?;
        let testo = clipboard.get_text().map_err(|e| e.to_string())?;
        Ok(ripulisci_testo(&testo))
    }

    /// Avvia la lettura vocale di un testo in un processo PowerShell nascosto.
    pub fn parla(&mut self, testo: &str) {
        self.parla_con_velocita(testo, VelocitaVoce::Normale);
    }

    /// Avvia la lettura vocale scegliendo la velocita'.
    ///
    /// Il testo passa da stdin: niente problemi di quoting con i caratteri
    /// speciali, che sono frequenti negli appunti accademici.
    ///
    /// Se la voce e' disattivata non parte nulla: chi studia nel silenzio non
    /// deve sentirsi comparire delle voci dal rumore di sottofondo.
    pub fn parla_con_velocita(&mut self, testo: &str, velocita: VelocitaVoce) {
        self.ferma_voce();
        if !self.voce_attiva {
            return;
        }
        let testo = testo.trim();
        if testo.is_empty() {
            return;
        }
        let script = script_tts(velocita.rate());
        let mut child = match Command::new("powershell.exe")
            .args([
                "-NoProfile",
                "-NonInteractive",
                "-WindowStyle",
                "Hidden",
                "-ExecutionPolicy",
                "Bypass",
                "-Command",
                &script,
            ])
            .creation_flags(CREATE_NO_WINDOW)
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
        {
            Ok(c) => c,
            Err(e) => {
                let _ = self.tx.send(Voce::Errore(format!("Voce non disponibile: {e}")));
                return;
            }
        };
        if let Some(stdin) = child.stdin.as_mut() {
            let _ = stdin.write_all(testo.as_bytes());
        }
        // Chiude stdin: senza questo PowerShell aspetterebbe input per sempre.
        drop(child.stdin.take());
        let _ = self.tx.send(Voce::Iniziata);
        if let Ok(mut processi) = self.processi.lock() {
            processi.push(child);
        }
        self.pulizia_processi();
    }

    /// Interrompe l'eventuale lettura vocale in corso.
    pub fn ferma_voce(&mut self) {
        let mut ha_fermato = false;
        if let Ok(mut processi) = self.processi.lock() {
            for processo in processi.iter_mut() {
                let _ = processo.kill();
                ha_fermato = true;
            }
            processi.clear();
        }
        if ha_fermato {
            let _ = self.tx.send(Voce::Interrotta);
        }
    }

    /// Elimina i processi vocali gia' terminati e notifica la fine lettura.
    pub(crate) fn pulizia_processi(&mut self) {
        let mut terminato = false;
        if let Ok(mut processi) = self.processi.lock() {
            processi.retain_mut(|p| match p.try_wait() {
                Ok(Some(_)) => {
                    terminato = true;
                    false
                }
                _ => true,
            });
        }
        if terminato {
            let _ = self.tx.send(Voce::Terminata);
        }
    }
}

/// Ripulisce un testo copiato da un PDF: toglie salti di riga e spazi doppi.
fn ripulisci_testo(testo: &str) -> String {
    let ripulito = testo.replace("\r\n", " ").replace('\n', " ");
    let mut risultato = String::with_capacity(ripulito.len());
    let mut spazio_precedente = false;
    for carattere in ripulito.chars() {
        if carattere.is_whitespace() {
            if !spazio_precedente {
                risultato.push(' ');
            }
            spazio_precedente = true;
        } else {
            risultato.push(carattere);
            spazio_precedente = false;
        }
    }
    risultato.trim().to_string()
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn ripulisce_i_salti_di_riga_dei_pdf() {
        assert_eq!(
            ripulisci_testo("Capitolo 3\r\n\r\n  Il  diritto   romano"),
            "Capitolo 3 Il diritto romano"
        );
    }

    #[test]
    fn conserva_le_accentature_italiane() {
        assert_eq!(ripulisci_testo("Perché  così"), "Perché così");
    }

    #[test]
    fn le_velocita_voce_hanno_il_rate_giusto() {
        assert_eq!(VelocitaVoce::Normale.rate(), 1);
        assert_eq!(VelocitaVoce::Veloce.rate(), 3);
        assert_eq!(VelocitaVoce::Turbo.rate(), 5);
    }

    #[test]
    fn le_velocita_ruotano_in_ciclo() {
        assert_eq!(VelocitaVoce::Normale.successiva(), VelocitaVoce::Veloce);
        assert_eq!(VelocitaVoce::Veloce.successiva(), VelocitaVoce::Turbo);
        assert_eq!(VelocitaVoce::Turbo.precedente(), VelocitaVoce::Veloce);
    }
}
