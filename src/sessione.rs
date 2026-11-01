//! Sessione di studio: timer, pause e ciclo di vita.
//!
//! La sessione tiene conto delle pause: quando l'interleaving cambia testo,
//! il countdown principale si ferma davvero e non consuma tempo di studio.

use std::sync::mpsc::Sender;
use std::time::{Duration, Instant};

use crate::audio::MotoreAudio;
use crate::blocker::Segnale;
use crate::guardia::Guardia;
use crate::state::StatoApp;

/// Intervallo fra due richiami vocali di stato (15 minuti).
pub const INTERVALLO_RICHIAMO: Duration = Duration::from_secs(15 * 60);

/// Durata massima consentita per una sessione (in minuti).
pub const DURATA_MAXIMA: u32 = 180;

/// Durata di una pausa breve.
pub const DURATA_PAUSA: Duration = Duration::from_secs(5 * 60);

/// Dati della sessione di studio in corso.
#[derive(Clone)]
pub struct Sessione {
    /// Momento di inizio della sessione.
    pub inizio: Instant,
    /// Durata totale prevista.
    pub durata: Duration,
    /// Obiettivo della sessione.
    pub obiettivo: String,
    /// Ultimo richiamo vocale emesso.
    pub ultimo_richiamo: Duration,
    /// Momento in cui e' iniziata la pausa corrente, se in pausa.
    pub pausa_inizio: Option<Instant>,
    /// Tempo totale passato in pausa.
    pub pausa_totale: Duration,
}

impl Sessione {
    /// Crea una nuova sessione con la durata e l'obiettivo indicati.
    pub fn nuova(durata: Duration, obiettivo: String) -> Self {
        Self {
            inizio: Instant::now(),
            durata,
            obiettivo,
            ultimo_richiamo: Duration::ZERO,
            pausa_inizio: None,
            pausa_totale: Duration::ZERO,
        }
    }

    /// Tempo effettivamente trascorso, escluse le pause.
    pub fn trascorso(&self) -> Duration {
        let pause_correnti = self
            .pausa_inizio
            .map(|i| i.elapsed())
            .unwrap_or(Duration::ZERO);
        self.inizio
            .elapsed()
            .saturating_sub(self.pausa_totale)
            .saturating_sub(pause_correnti)
    }

    /// Tempo residuo alla fine della sessione.
    pub fn residuo(&self) -> Duration {
        self.durata.saturating_sub(self.trascorso())
    }

    /// Frazione di sessione gia' trascorsa (0.0 - 1.0).
    pub fn frazione(&self) -> f32 {
        let totale = self.durata.as_secs_f32().max(1.0);
        (self.trascorso().as_secs_f32() / totale).clamp(0.0, 1.0)
    }

    /// Vero se il tempo di studio e' finito.
    pub fn scaduta(&self) -> bool {
        self.trascorso() >= self.durata
    }

    /// Vero se il timer e' in pausa.
    pub fn in_pausa(&self) -> bool {
        self.pausa_inizio.is_some()
    }

    /// Ferma il timer, accumulando eventualmente una pausa gia' aperta.
    pub fn metti_in_pausa(&mut self) {
        if self.pausa_inizio.is_none() {
            self.pausa_inizio = Some(Instant::now());
        }
    }

    /// Riprende il timer dopo una pausa.
    pub fn riprendi(&mut self) {
        if let Some(inizio) = self.pausa_inizio.take() {
            self.pausa_totale += inizio.elapsed();
        }
    }
}

impl StatoApp {
    /// Avvia una sessione di studio e attiva il Livello 2 di blocco.
    ///
    /// Non tocca `avviso`: l'eventuale avviso giallo sui permessi mancanti e'
    /// impostato dalla GUI appena prima e deve restare visibile.
    pub fn avvia_sessione(&mut self, mittente: &Sender<Segnale>) {
        let durata = Duration::from_secs(self.config.durata_minuti as u64 * 60);
        self.sessione = Some(Sessione::nuova(durata, self.config.obiettivo.clone()));
        // Il punto di partenza degli XP: lo storico registra il guadagno
        // della sessione, non il totale accumulato da sempre.
        self.xp_a_inizio = (self.xp.punti, self.xp.distrazioni);
        self.tempo_documenti.clear();
        self.ultimo_campione = Instant::now();
        let parole = crate::config::righe_alla_lista(&self.testo_parole);
        // La guardia deve sapere quali testi appartengono all'esame scelto.
        self.guardia = Some(Guardia::avvia(parole, self.testi_scelti.clone(), mittente.clone()));
    }

    /// Termina la sessione, ferma il Livello 2 e ripristina il file hosts.
    pub fn termina_sessione(&mut self) {
        // Prima di buttare la sessione, la si annota nello storico.
        if let Some(s) = self.sessione.take() {
            let minuti = (s.trascorso().as_secs() / 60) as u32;
            if minuti >= 1 {
                if let Some(record) = self.sessione_da_registrare(minuti) {
                    crate::storico::registra(record);
                }
            }
        }
        if let Some(mut guardia) = self.guardia.take() {
            guardia.ferma();
        }
        crate::blocker::ripristina_hosts();
        crate::blocker::flush_dns();
    }

    /// Costruisce la riga di storico di una sessione appena conclusa.
    ///
    /// XP e distrazioni sono la differenza rispetto all'inizio della
    /// sessione: `self.xp` e' il totale di sempre, e scriverlo qui faceva
    /// sommare due volte lo stesso lavoro nel riepilogo.
    fn sessione_da_registrare(&self, minuti: u32) -> Option<crate::storico::SessioneRegistrata> {
        let (xp_inizio, distrazioni_inizio) = self.xp_a_inizio;
        Some(crate::storico::SessioneRegistrata {
            data: crate::storico::adesso(),
            esame: self.nome_esame(),
            minuti,
            xp: (self.xp.punti - xp_inizio).max(0),
            distrazioni: self.xp.distrazioni.saturating_sub(distrazioni_inizio),
            blocchi: self.task.completati,
            tempo_documenti: self.tempo_documenti.clone(),
        })
    }

    /// Mette la sessione in pausa per una durata fissa.
    ///
    /// La pausa pianificata e' diversa dallo sblocco d'emergenza: serve a
    /// fermarsi cinque minuti senza abbandonare, ed e' quello che tiene in
    /// piedi una sessione lunga.
    pub fn metti_in_pausa(&mut self) {
        if let Some(s) = self.sessione.as_mut() {
            s.metti_in_pausa();
            self.pausa_scade = Some(Instant::now() + DURATA_PAUSA);
        }
    }

    /// Toglie la pausa e riprende il conteggio.
    pub fn riprendi_sessione(&mut self) {
        self.pausa_scade = None;
        if let Some(s) = self.sessione.as_mut() {
            s.riprendi();
        }
    }

    /// Secondi che mancano alla fine della pausa, se c'e' una pausa attiva.
    pub fn pausa_residua(&self) -> Option<u64> {
        let scadenza = self.pausa_scade?;
        Some(
            scadenza
                .saturating_duration_since(Instant::now())
                .as_secs(),
        )
    }

    /// Chiude la pausa quando il tempo e' scaduto.
    fn controlla_pausa(&mut self, audio: &mut MotoreAudio) {
        let Some(scadenza) = self.pausa_scade else {
            return;
        };
        if Instant::now() < scadenza {
            return;
        }
        self.riprendi_sessione();
        audio.beep();
        audio.parla("Pausa finita. Si riparte.");
        self.aggiungi_notifica("Pausa finita: si riparte.");
    }

    /// Fa avanzare la sessione: richiami vocali e chiusura a tempo scaduto.
    pub fn aggiorna_sessione(&mut self, audio: &mut MotoreAudio) {
        self.controlla_pausa(audio);

        let (scaduta, richiamo) = match &self.sessione {
            Some(s) => {
                let scaduta = s.scaduta();
                let richiamo = self.config.richiamo_vocale
                    && s.trascorso() >= s.ultimo_richiamo + INTERVALLO_RICHIAMO;
                (scaduta, richiamo)
            }
            None => (false, false),
        };

        if let Some(s) = self.sessione.as_mut() {
            if richiamo {
                s.ultimo_richiamo = s.trascorso();
                let minuti = s.trascorso().as_secs() / 60;
                let obiettivo = s.obiettivo.clone();
                let frase =
                    format!("Sono passati {minuti} minuti. Stai procedendo bene su {obiettivo}");
                audio.parla(&frase);
            }
        }

        if scaduta {
            let obiettivo = self.obiettivo_corrente().to_string();
            audio.parla(&format!("Sessione completata. Ottimo lavoro su {obiettivo}."));
            audio.ferma_sottofondo();
            self.termina_sessione();
            self.aggiungi_notifica("Sessione completata. Prenditi una pausa.");
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    /// Lo storico deve registrare il guadagno della sessione, non il totale
    /// di sempre: scrivendo il totale, il riepilogo sommava due volte lo
    /// stesso lavoro.
    #[test]
    fn lo_storico_registra_solo_il_guadagno_della_sessione() {
        let mut stato = StatoApp::nuovo();
        stato.xp.punti = 300;
        stato.xp.distrazioni = 2;
        stato.xp_a_inizio = (300, 2);

        // Durante la sessione: +150 XP, poi una distrazione sventata (-25).
        stato.xp.aggiungi(150);
        stato.xp.registra_distrazione();
        assert_eq!(stato.xp.punti, 425);

        let record = stato.sessione_da_registrare(25).expect("record");
        assert_eq!(record.minuti, 25);
        assert_eq!(record.xp, 125);
        assert_eq!(record.distrazioni, 1);
    }

    #[test]
    fn la_pausa_non_consuma_tempo_di_studio() {
        let mut s = Sessione::nuova(Duration::from_secs(60), "Test".to_string());
        s.metti_in_pausa();
        std::thread::sleep(Duration::from_millis(30));
        // Il tempo trascorso non deve crescere durante la pausa.
        let durante = s.trascorso();
        std::thread::sleep(Duration::from_millis(30));
        assert!(s.trascorso() <= durante + Duration::from_millis(5));
        s.riprendi();
        assert!(!s.in_pausa());
        assert!(s.residuo() > Duration::ZERO);
    }

    #[test]
    fn la_frazione_resta_tra_zero_e_uno() {
        let s = Sessione::nuova(Duration::from_secs(1), "Test".to_string());
        assert!((0.0..=1.0).contains(&s.frazione()));
    }
}
