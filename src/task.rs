//! Micro-task di studio con premio immediato.
//!
//! Il problema dell'ADHD non e' la mancanza di tempo, ma la difficolta' a
//! iniziare e a restare agganciati a un impegno lungo. Qui si rompe lo studio
//! in blocchetti da 1, 2, 3, 5 e 10 minuti: ogni blocco e' piccolo abbastanza
//! da sembrare incurabile, e finisce con un premio subito.
//!
//! Il premio non e' solo un numero: la voce lo annuncia e l'interfaccia lo
//! mette in evidenza. Il segnale di ricompensa deve arrivare subito, non
//! fra venti minuti.

use std::time::{Duration, Instant};

/// Durate dei micro-task, in minuti.
pub const DURATE: [u32; 5] = [1, 2, 3, 5, 10];

/// Punti XP guadagnati per ogni minuto di micro-task completato.
///
/// Il premio e' proporzionale ma con un piccolo bonus: conviene scegliere
/// blocchi piu' lunghi, senza pero' renderli irraggiungibili.
pub const XP_PER_MINUTO: i32 = 12;

/// Bonus extra per aver completato un micro-task.
pub const BONUS_COMPLETAMENTO: i32 = 10;

/// Un micro-task in corso.
#[derive(Clone, Debug)]
pub struct Task {
    /// Durata richiesta, in minuti.
    pub minuti: u32,
    /// Momento di inizio.
    pub inizio: Instant,
    /// Punti XP che verranno assegnati al completamento.
    pub premio: i32,
}

/// Micro-task in esecuzione.
#[derive(Clone, Debug, Default)]
pub struct GestoreTask {
    /// Task attualmente in corso, se presente.
    pub corrente: Option<Task>,
    /// Numero di micro-task completati nella sessione.
    pub completati: u32,
}

impl GestoreTask {
    /// Crea un gestore vuoto.
    pub fn nuovo() -> Self {
        Self::default()
    }

    /// Avvia un micro-task della durata indicata.
    pub fn avvia(&mut self, minuti: u32) -> Task {
        let task = Task {
            minuti,
            inizio: Instant::now(),
            premio: premio_per(minuti),
        };
        self.corrente = Some(task.clone());
        task
    }

    /// Secondi rimasti al micro-task in corso.
    pub fn secondi_residui(&self) -> Option<u64> {
        let task = self.corrente.as_ref()?;
        let totale = Duration::from_secs(task.minuti as u64 * 60);
        Some(totale.as_secs().saturating_sub(task.inizio.elapsed().as_secs()))
    }

    /// Vero se il micro-task in corso e' terminato.
    pub fn e_completato(&self) -> bool {
        match &self.corrente {
            Some(task) => {
                task.inizio.elapsed() >= Duration::from_secs(task.minuti as u64 * 60)
            }
            None => false,
        }
    }

    /// Frazione di tempo trascorsa (0.0 - 1.0).
    pub fn frazione(&self) -> f32 {
        match &self.corrente {
            Some(task) => {
                let totale = (task.minuti as f32 * 60.0).max(1.0);
                (task.inizio.elapsed().as_secs_f32() / totale).clamp(0.0, 1.0)
            }
            None => 0.0,
        }
    }

    /// Chiude il micro-task e restituisce i punti XP guadagnati.
    pub fn completa(&mut self) -> Option<i32> {
        let task = self.corrente.take()?;
        self.completati += 1;
        Some(task.premio)
    }

    /// Annulla il micro-task in corso senza premio.
    pub fn annulla(&mut self) {
        self.corrente = None;
    }
}

/// Calcola il premio per un micro-task della durata indicata.
pub fn premio_per(minuti: u32) -> i32 {
    minuti as i32 * XP_PER_MINUTO + BONUS_COMPLETAMENTO
}

/// Etichetta breve del micro-task, per l'interfaccia.
pub fn etichetta(minuti: u32) -> String {
    format!("{minuti} min")
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn il_premio_cresce_con_la_durata() {
        assert!(premio_per(1) < premio_per(2));
        assert!(premio_per(2) < premio_per(5));
        assert!(premio_per(5) < premio_per(10));
    }

    #[test]
    fn il_task_si_completa_al_termine_del_timer() {
        let mut gestore = GestoreTask::nuovo();
        gestore.avvia(1);
        // Un task da 1 minuto non e' ancora scaduto.
        assert!(!gestore.e_completato());
        assert!(gestore.secondi_residui().unwrap() > 0);
    }

    #[test]
    fn la_completazione_assegna_il_premio_e_azzera() {
        let mut gestore = GestoreTask::nuovo();
        gestore.avvia(3);
        let premio = gestore.completa();
        assert_eq!(premio, Some(premio_per(3)));
        assert_eq!(gestore.completati, 1);
        assert!(gestore.corrente.is_none());
        assert_eq!(gestore.secondi_residui(), None);
    }

    #[test]
    fn l_annullamento_non_da_premio() {
        let mut gestore = GestoreTask::nuovo();
        gestore.avvia(2);
        gestore.annulla();
        assert_eq!(gestore.completati, 0);
        assert!(gestore.corrente.is_none());
    }
}
