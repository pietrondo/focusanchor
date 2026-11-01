//! Logica anti-noia e anti-sonno.
//!
//! Due meccanismi complementari per chi legge in modo passivo:
//! - il protocollo "S.O.S. Sonnolenza": micro-missioni fisiche da 60 secondi;
//! - le "Domande a Tradimento": interrogazioni a sorpresa sul testo appena
//!   letto, con risposta da scrivere (active recall).

use std::time::{Duration, Instant};

use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};

/// Durata del protocollo S.O.S. Sonnolenza.
pub const DURATA_MISSIONE: Duration = Duration::from_secs(60);

/// Intervallo minimo fra due domande a tradimento.
pub const INTERVALLO_MINIMO: Duration = Duration::from_secs(8 * 60);

/// Intervallo massimo fra due domande a tradimento.
pub const INTERVALLO_MASSIMO: Duration = Duration::from_secs(14 * 60);

/// Lunghezza minima della risposta all'interrogazione (in caratteri).
pub const RISPOSTA_MINIMA: usize = 30;

/// Punti XP guadagnati completando una missione fisica.
pub const XP_MISSIONE: i32 = 40;

/// Punti XP guadagnati rispondendo a una domanda a tradimento.
pub const XP_DOMANDA: i32 = 50;

/// Tipo di sfida attualmente attiva.
#[derive(Clone, Debug)]
pub enum Sfida {
    /// Nessuna sfida in corso.
    Nessuna,
    /// Micro-missione fisica da 60 secondi.
    Missione {
        /// Testo della missione da eseguire.
        testo: String,
        /// Momento di inizio della missione.
        inizio: Instant,
    },
    /// Domanda a tradimento: ricordare e riassumere un passaggio.
    Domanda {
        /// Frase da ricordare (sottolineatura di Zotero).
        passaggio: String,
        /// Momento di inizio della sfida.
        inizio: Instant,
    },
}

/// Micro-missioni fisiche per scuotere la sonnolenza.
///
/// Sono brevi, non richiedono attrezzature e agiscono sul sistema nervoso
/// parasimpatico: il movimento e lo sguardo lontano dallo schermo
/// riattivano la vigilanza.
pub const MISSIONI: [&str; 6] = [
    "Alzati in piedi! Fai 12 tirate orizzontali o rematori e bevi un sorso d'acqua.",
    "Fai 15 box squat sul posto per riattivare la circolazione.",
    "Alzati, cammina velocemente per la stanza per 45 secondi e guarda fuori dalla finestra lontano dallo schermo.",
    "Fai 20 jumping jack a braccia aperte, poi torna a sederti lentamente.",
    "Stacca le spalle dal fondo della sedia: tieni la schiena dritta per 30 secondi.",
    "Guarda lontano dallo schermo per 20 secondi, poi strizza gli occhi 10 volte.",
];

/// Stato del modulo anti-sonno.
pub struct AntiSonno {
    /// Sfida attualmente attiva.
    pub sfida: Sfida,
    /// Momento della prossima domanda a tradimento.
    prossima_domanda: Instant,
    /// Prossima domanda da porre (se ci sono sottolineature Zotero).
    prossima_passaggio: Option<String>,
    /// Testo scritto dall'utente per la domanda a tradimento.
    pub risposta: String,
    /// Sfide a sorpresa abilitate.
    pub attivo: bool,
}

impl AntiSonno {
    /// Crea il modulo e calcola la prima domanda a tradimento.
    pub fn nuovo(attivo: bool) -> Self {
        Self {
            sfida: Sfida::Nessuna,
            prossima_domanda: Instant::now() + intervallo_casuale(),
            prossima_passaggio: None,
            risposta: String::new(),
            attivo,
        }
    }

    /// Estrae una nuova missione casuale e avvia il timer da 60 secondi.
    pub fn avvia_missione(&mut self) -> String {
        let mut rng = StdRng::from_entropy();
        let testo = MISSIONI[rng.gen_range(0..MISSIONI.len())].to_string();
        self.sfida = Sfida::Missione {
            testo: testo.clone(),
            inizio: Instant::now(),
        };
        self.risposta.clear();
        testo
    }

    /// Imposta il testo della prossima domanda a tradimento.
    pub fn imposta_passaggio(&mut self, passaggio: Option<String>) {
        self.prossima_passaggio = passaggio;
    }

    /// Restituisce `true` se e' ora di porre una domanda a tradimento.
    pub fn domanda_da_porre(&self) -> bool {
        self.attivo && matches!(self.sfida, Sfida::Nessuna) && Instant::now() >= self.prossima_domanda
    }

    /// Avvia la domanda a tradimento, se c'e' un passaggio da chiedere.
    pub fn avvia_domanda(&mut self) -> Option<String> {
        let passaggio = self.prossima_passaggio.clone()?;
        self.sfida = Sfida::Domanda {
            passaggio: passaggio.clone(),
            inizio: Instant::now(),
        };
        self.risposta.clear();
        Some(passaggio)
    }

    /// Segna la sfida come conclusa e riprogramma la prossima domanda.
    pub fn completa_sfida(&mut self) {
        self.sfida = Sfida::Nessuna;
        self.risposta.clear();
        self.prossima_domanda = Instant::now() + intervallo_casuale();
    }

    /// Secondi rimasti alla missione fisica.
    pub fn secondi_residui(&self) -> Option<u64> {
        match &self.sfida {
            Sfida::Missione { inizio, .. } => {
                let passati = inizio.elapsed().as_secs();
                Some(DURATA_MISSIONE.as_secs().saturating_sub(passati))
            }
            _ => None,
        }
    }

    /// Secondi trascorsi dall'inizio della sfida corrente.
    ///
    /// Nella domanda a tradimento serve a dare una sensazione di tempo che
    /// scorre: sprona a rispondere di getto, prima di rileggere il testo.
    pub fn secondi_trascorsi(&self) -> u64 {
        match &self.sfida {
            Sfida::Missione { inizio, .. } | Sfida::Domanda { inizio, .. } => {
                inizio.elapsed().as_secs()
            }
            Sfida::Nessuna => 0,
        }
    }

    /// Verifica che la risposta scritta sia abbastanza lunga per valere.
    pub fn risposta_valida(&self) -> bool {
        self.risposta.trim().chars().count() >= RISPOSTA_MINIMA
    }
}

/// Calcola un intervallo casuale fra 8 e 14 minuti.
fn intervallo_casuale() -> Duration {
    let mut rng = StdRng::from_entropy();
    let secondi = rng.gen_range(INTERVALLO_MINIMO.as_secs()..=INTERVALLO_MASSIMO.as_secs());
    Duration::from_secs(secondi)
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn l_intervallo_e_compreso_tra_8_e_14_minuti() {
        for _ in 0..50 {
            let i = intervallo_casuale();
            assert!(i >= INTERVALLO_MINIMO && i <= INTERVALLO_MASSIMO);
        }
    }

    #[test]
    fn la_missione_estratta_e_una_delle_missioni_note() {
        let mut anti = AntiSonno::nuovo(false);
        let testo = anti.avvia_missione();
        assert!(MISSIONI.contains(&testo.as_str()));
    }

    #[test]
    fn la_risposta_troppo_corta_non_e_valida() {
        let mut anti = AntiSonno::nuovo(false);
        anti.risposta = "troppo corto".to_string();
        assert!(!anti.risposta_valida());
        anti.risposta = "x".repeat(RISPOSTA_MINIMA);
        assert!(anti.risposta_valida());
    }
}
