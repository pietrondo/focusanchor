//! Interleaving: rotazione rapida fra piu' testi dello stesso esame.
//!
//! Studiare un solo testo per ore abbassa l'attenzione. Alternare due o tre
//! testi ogni 10-12 minuti tiene alto l'interesse e costringe il cervello a
//! recuperare il contesto: e' il principio dell'interleaving.
//!
//! Allo scadere del blocco il countdown principale si mette in pausa e
//! l'utente deve prendersi un respiro prima di ripartire sul testo nuovo.

use std::time::{Duration, Instant};

use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};

use crate::audio::MotoreAudio;
use crate::state::StatoApp;

/// Durata minima di un blocco di studio su un testo.
pub const BLOCCO_MINIMO: Duration = Duration::from_secs(10 * 60);

/// Durata massima di un blocco di studio su un testo.
pub const BLOCCO_MASSIMO: Duration = Duration::from_secs(12 * 60);

/// Rotazione circolare fra i testi selezionati.
#[derive(Clone, Debug)]
pub struct Interleaving {
    /// Titoli dei testi da alternare.
    pub testi: Vec<String>,
    /// Indice del testo attualmente in studio.
    pub indice: usize,
    /// Durata del blocco corrente.
    pub durata_blocco: Duration,
    /// Momento di inizio del blocco corrente.
    pub inizio_blocco: Instant,
    /// Vero quando il blocco e' scaduto e si aspetta il respiro dell'utente.
    pub in_pausa: bool,
}

impl Interleaving {
    /// Crea la rotazione con blocchi di durata casuale fra 10 e 12 minuti.
    ///
    /// La durata varia da un blocco all'altro: un intervallo fisso diventa
    /// prevedibile e il cervello smette di prestarci attenzione.
    pub fn nuovo(testi: Vec<String>) -> Self {
        Self {
            testi,
            indice: 0,
            durata_blocco: durata_casuale(),
            inizio_blocco: Instant::now(),
            in_pausa: false,
        }
    }

    /// Vero se la rotazione e' attiva (almeno due testi).
    pub fn attivo(&self) -> bool {
        self.testi.len() >= 2
    }

    /// Titolo del testo attualmente in studio.
    pub fn testo_corrente(&self) -> Option<&String> {
        self.testi.get(self.indice)
    }

    /// Secondi rimasti al cambio di testo, se la rotazione e' attiva.
    pub fn secondi_residui(&self) -> Option<u64> {
        if !self.attivo() || self.in_pausa {
            return None;
        }
        let passato = self.inizio_blocco.elapsed().as_secs();
        Some(self.durata_blocco.as_secs().saturating_sub(passato))
    }

    /// Vero se il blocco corrente e' scaduto e va fatto il cambio.
    pub fn blocco_scaduto(&self) -> bool {
        self.attivo() && !self.in_pausa && self.inizio_blocco.elapsed() >= self.durata_blocco
    }

    /// Passa al testo successivo e mette la rotazione in pausa.
    ///
    /// Restituisce il titolo del testo nuovo, da annunciare a voce.
    pub fn passa_al_successivo(&mut self) -> Option<String> {
        if !self.attivo() {
            return None;
        }
        self.indice = (self.indice + 1) % self.testi.len();
        self.in_pausa = true;
        self.testi.get(self.indice).cloned()
    }

    /// Riprende la rotazione dopo il respiro, ripartendo da capo col timer.
    pub fn riprendi(&mut self) {
        self.in_pausa = false;
        self.inizio_blocco = Instant::now();
        // Ogni blocco ha una durata nuova, cosi' il cambio non e' prevedibile.
        self.durata_blocco = durata_casuale();
    }

    /// Frazione trascorsa del blocco corrente (0.0 - 1.0).
    pub fn frazione(&self) -> f32 {
        if !self.attivo() {
            return 0.0;
        }
        let totale = self.durata_blocco.as_secs_f32().max(1.0);
        (self.inizio_blocco.elapsed().as_secs_f32() / totale).clamp(0.0, 1.0)
    }
}

impl StatoApp {
    /// Controlla se e' ora di cambiare testo e, se serve, fa la rotazione.
    ///
    /// Allo scadere del blocco il countdown principale va in pausa: l'utente
    /// deve prendersi un respiro prima di ripartire sul testo nuovo.
    pub fn aggiorna_interleaving(&mut self, audio: &mut MotoreAudio) {
        if !self.interleaving.blocco_scaduto() {
            return;
        }
        let precedente = self
            .interleaving
            .testo_corrente()
            .cloned()
            .unwrap_or_default();
        let prossimo = match self.interleaving.passa_al_successivo() {
            Some(t) => t,
            None => return,
        };
        // Il countdown si ferma davvero: il cambio testo e' un respiro.
        if let Some(s) = self.sessione.as_mut() {
            s.metti_in_pausa();
        }
        audio.beep();
        audio.parla(&format!(
            "Tempo scaduto su {precedente}! Fai un respiro e passa al testo successivo: {prossimo}"
        ));
        self.aggiungi_notifica(&format!("Ora studia: {prossimo}"));
    }

    /// Riprende la sessione dopo il respiro fra due testi.
    pub fn riprendi_dopo_cambio(&mut self) {
        self.interleaving.riprendi();
        if let Some(s) = self.sessione.as_mut() {
            s.riprendi();
        }
    }
}

/// Calcola una durata di blocco casuale fra 10 e 12 minuti.
fn durata_casuale() -> Duration {
    let mut rng = StdRng::from_entropy();
    let minimo = BLOCCO_MINIMO.as_secs();
    let massimo = BLOCCO_MASSIMO.as_secs();
    Duration::from_secs(rng.gen_range(minimo..=massimo))
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn con_un_solo_testo_l_interleaving_non_e_attivo() {
        let i = Interleaving::nuovo(vec!["Solo uno".to_string()]);
        assert!(!i.attivo());
        assert!(i.secondi_residui().is_none());
        assert!(!i.blocco_scaduto());
    }

    #[test]
    fn la_rotazione_torna_al_primo_testo() {
        let mut i = Interleaving::nuovo(vec![
            "Primo".to_string(),
            "Secondo".to_string(),
            "Terzo".to_string(),
        ]);
        assert_eq!(i.testo_corrente().unwrap(), "Primo");
        assert_eq!(i.passa_al_successivo().unwrap(), "Secondo");
        assert_eq!(i.passa_al_successivo().unwrap(), "Terzo");
        assert_eq!(i.passa_al_successivo().unwrap(), "Primo");
    }

    #[test]
    fn il_cambio_manda_in_pausa_finche_non_si_riprende() {
        let mut i = Interleaving::nuovo(vec!["A".to_string(), "B".to_string()]);
        i.passa_al_successivo();
        assert!(i.in_pausa);
        assert!(i.secondi_residui().is_none());
        i.riprendi();
        assert!(!i.in_pausa);
        assert!(i.secondi_residui().is_some());
    }
}
