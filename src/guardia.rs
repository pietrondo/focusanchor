//! Guardia delle finestre attive (Livello 2 di blocco).
//!
//! Un thread dedicato controlla ogni 800 ms il titolo della finestra in primo
//! piano e fa due cose:
//!
//! - se il titolo contiene una parola vietata, avvisa: niente viene chiuso,
//!   l'app dice solo a voce quale era l'obiettivo;
//! - se e' una scheda di Zotero con un PDF di un ALTRO esame, avvisa: studiare
//!   un corso diverso e' un cambio di contesto che costa attenzione.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Sender;
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use crate::blocker::{titolo_finestra_attiva, Segnale};
use crate::finestre::SUFFISSO_ZOTERO;
use crate::zotero::appartiene_all_esame;

/// Intervallo tra due controlli della finestra in primo piano.
pub const INTERVALLO_CONTROLLO: Duration = Duration::from_millis(800);

/// Durata dei singoli sonnellini: piu' breve del controllo, cosi' la guardia
/// si ferma subito quando l'utente chiude la sessione.
const INTERVALLO_SONNO: Duration = Duration::from_millis(50);

/// Tempo minimo fra due avvisi di cambio contesto.
///
/// Senza questa pausa l'avviso vocale ripartirebbe ogni 800 ms e diventerebbe
/// lui stesso una distrazione.
pub const COOLDOWN_CONTESTO: Duration = Duration::from_secs(15);

/// Tempo minimo fra due avvisi di distrazione.
///
/// Una distrazione costa 25 XP: la stessa finestra non va penalizzata ogni
/// 800 ms mentre resta in primo piano.
pub const COOLDOWN_DISTRAZIONE: Duration = Duration::from_secs(30);

/// Ultimi avvisi mandati, per non ripeterli.
#[derive(Default)]
struct Avvisi {
    /// Ultima distrazione segnalata, con il titolo della finestra.
    distrazione: Option<(String, Instant)>,
    /// Ultimo avviso di cambio di contesto.
    contesto: Option<Instant>,
}

/// Guardia attiva che sorveglia le finestre in primo piano.
pub struct Guardia {
    /// Flag di arresto chiesto dalla GUI.
    ferma: Arc<AtomicBool>,
    /// Handle del thread di sorveglianza.
    thread: Option<JoinHandle<()>>,
}

impl Guardia {
    /// Avvia la guardia.
    ///
    /// `parole` sono le parole vietate nei titoli; `testi_esame` sono i titoli
    /// dei documenti dell'esame in corso: un PDF di Zotero che non appartiene
    /// a questa lista fa scattare l'avviso di cambio contesto.
    pub fn avvia(
        parole: Vec<String>,
        testi_esame: Vec<String>,
        canale: Sender<Segnale>,
    ) -> Self {
        let ferma = Arc::new(AtomicBool::new(false));
        let flag = Arc::clone(&ferma);
        let thread = thread::spawn(move || {
            let mut ultimo_controllo = Instant::now();
            let mut avvisi = Avvisi::default();
            loop {
                thread::sleep(INTERVALLO_SONNO);
                if flag.load(Ordering::Relaxed) {
                    break;
                }
                if ultimo_controllo.elapsed() < INTERVALLO_CONTROLLO {
                    continue;
                }
                ultimo_controllo = Instant::now();
                controlla_una_volta(&parole, &testi_esame, &canale, &mut avvisi);
            }
        });
        Self {
            ferma,
            thread: Some(thread),
        }
    }

    /// Ferma la guardia e attende la fine del thread.
    pub fn ferma(&mut self) {
        self.ferma.store(true, Ordering::Relaxed);
        if let Some(handle) = self.thread.take() {
            let _ = handle.join();
        }
    }
}

impl Drop for Guardia {
    fn drop(&mut self) {
        self.ferma();
    }
}

/// Esegue un singolo controllo sulla finestra attiva.
fn controlla_una_volta(
    parole: &[String],
    testi_esame: &[String],
    canale: &Sender<Segnale>,
    avvisi: &mut Avvisi,
) {
    // La finestra di FocusAnchor non deve mai essere minimizzata da se' stessa.
    if crate::blocker::finestra_è_di_focus_anchor() {
        return;
    }
    let titolo = titolo_finestra_attiva();
    if titolo.is_empty() {
        return;
    }

    // 1) Finestra che fa perdere il filo: si avvisa, senza chiudere nulla.
    //    Il blocco dei siti e' stato tolto su richiesta: l'app non deve
    //    minimizzare le finestre dell'utente, solo ricordargli l'obiettivo.
    if crate::blocker::titolo_vietato(&titolo, parole) {
        let adesso = Instant::now();
        // La stessa finestra non viene penalizzata due volte: restare
        // davanti a una scheda distratta non e' un'altra distrazione.
        let ripetuta = avvisi
            .distrazione
            .as_ref()
            .map(|(visto, istante)| {
                *visto == titolo || adesso.duration_since(*istante) < COOLDOWN_DISTRAZIONE
            })
            .unwrap_or(false);
        if !ripetuta {
            avvisi.distrazione = Some((titolo.clone(), adesso));
            let _ = canale.send(Segnale::Distrazione(titolo));
        }
        return;
    }

    // 2) Cambio di contesto dentro Zotero: si avvisa, senza pausa.
    if let Some(documento) = documento_zotero(&titolo) {
        if testi_esame.is_empty() {
            return;
        }
        if appartiene_all_esame(&documento, testi_esame) {
            return;
        }
        // Rispetta il cooldown: dopo un avviso, aspetta prima del successivo.
        let adesso = Instant::now();
        let troppobuono = avvisi
            .contesto
            .map(|ultimo| adesso.duration_since(ultimo) < COOLDOWN_CONTESTO)
            .unwrap_or(false);
        if troppobuono {
            return;
        }
        avvisi.contesto = Some(adesso);
        let _ = canale.send(Segnale::ContestoZotero(documento));
    }
}

/// Estrae il nome del documento dal titolo di una scheda di Zotero.
fn documento_zotero(titolo: &str) -> Option<String> {
    titolo
        .trim()
        .strip_suffix(SUFFISSO_ZOTERO)
        .map(|nome| nome.trim().to_string())
        .filter(|nome| !nome.is_empty())
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn estrae_il_documento_dal_titolo_di_zotero() {
        assert_eq!(
            documento_zotero("Storia economica - Zotero").unwrap(),
            "Storia economica"
        );
        assert!(documento_zotero("Documento senza suffisso").is_none());
        assert!(documento_zotero(" - Zotero").is_none());
    }
}
