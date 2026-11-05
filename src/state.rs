//! Stato dell'applicazione: esame scelto, testi, XP e archivio.

use std::path::PathBuf;
use std::time::Instant;

use crate::anti_sleep::AntiSonno;
use crate::archivi;
use crate::audio::{ModalitaAudio, MotoreAudio, Voce};
use crate::audio_voce::VelocitaVoce;
use crate::config::{
    carica_configurazione, lista_alle_righe, righe_alla_lista, salva_configurazione,
    Configurazione,
};
use crate::guardia::Guardia;
use crate::interleaving::Interleaving;
use crate::sessione::Sessione;
use crate::xp::PuntiXp;
use crate::zotero::{Collezione, Documento, Evidenziazione};

pub use crate::sessione::DURATA_MAXIMA;

/// Numero massimo di testi alternabili in una sessione.
pub const MAX_TESTI: usize = 3;

/// Stato completo dell'applicazione.
pub struct StatoApp {
    /// Configurazione corrente.
    pub config: Configurazione,
    /// Sessione attiva, se presente.
    pub sessione: Option<Sessione>,
    /// Guardia finestre attiva durante la sessione.
    pub guardia: Option<Guardia>,
    /// Punti XP e grado di concentrazione.
    pub xp: PuntiXp,
    /// XP e distrazioni all'inizio della sessione in corso.
    ///
    /// Servono a scrivere nello storico il guadagno della sessione e non il
    /// totale di sempre, che verrebbe sommato una seconda volta.
    pub xp_a_inizio: (i32, u32),
    /// Secondi passati su ogni documento durante la sessione in corso.
    ///
    /// La chiave e' il titolo del documento (o `tempo::CHIAVE_RIASSUNTI` per
    /// i riassunti aperti fuori da Zotero).
    pub tempo_documenti: std::collections::BTreeMap<String, u32>,
    /// Ultimo istante del campionamento della finestra in primo piano.
    pub ultimo_campione: Instant,
    /// Carte di ripasso dell'esame scelto.
    pub memoria: crate::carte::MemoriaEsame,
    /// Esame a cui appartiene `memoria`.
    pub memoria_esame: String,
    /// Carta in ripasso adesso: indice dentro `memoria.carte`.
    pub ripasso_indice: Option<usize>,
    /// Vero quando la risposta della carta e' stata mostrata.
    pub ripasso_risposta_visibile: bool,
    /// Ultimo comando arrivato da Zotero, per mostrarlo nel pannello.
    pub ultimo_comando: String,
    /// Modulo anti-noia e anti-sonno.
    pub anti_sonno: AntiSonno,
    /// Micro-task di studio in corso.
    pub task: crate::task::GestoreTask,
    /// Assistente AI per spiegare i passaggi ostici.
    pub ai: crate::ai::Assistente,
    /// Sessioni concluse, lette da disco una volta sola.
    pub storico: Vec<crate::storico::SessioneRegistrata>,
    /// Riepilogo dell'ultima sessione terminata, con dati effettivamente registrati.
    pub ultimo_riepilogo: Option<crate::storico::SessioneRegistrata>,
    /// Collezioni Zotero, cioe' gli esami disponibili.
    pub collezioni: Vec<Collezione>,
    /// Esame scelto (identificativo della collezione).
    pub esame_scelto: Option<i64>,
    /// Testi dell'esame scelto.
    pub documenti_esame: Vec<Documento>,
    /// Testi spuntati per la sessione a blocchi incrociati.
    pub testi_scelti: Vec<String>,
    /// Rotazione fra i testi scelti.
    pub interleaving: Interleaving,
    /// Momento in cui finisce la pausa in corso.
    ///
    /// Lo gestisce `sessione.rs`: qui serve solo come stato condiviso.
    pub pausa_scade: Option<Instant>,
    /// Sottolineature recenti lette da Zotero.
    pub evidenziazioni: Vec<Evidenziazione>,
    /// Percorso dello snapshot del database Zotero.
    pub snapshot_zotero: Option<PathBuf>,
    /// Testo in modifica nel campo del link NotebookLM.
    ///
    /// Deve stare nello stato: un buffer locale verrebbe ricreato a ogni
    /// frame e l'incolla sparirebbe subito.
    pub campo_notebook: String,
    /// Anteprima della scheda NotebookLM aperta nella schermata Materiali.
    pub anteprima_notebooklm: Option<(String, String)>,
    /// Testo del campo "parcheggio pensieri".
    pub campo_pensiero: String,
    /// Domanda libera da mandare a NotebookLM per l'esame scelto.
    ///
    /// Sta nello stato perche' un buffer locale verrebbe ricreato a ogni
    /// frame e la domanda digitata sparirebbe.
    pub campo_domanda: String,
    /// Domanda effettivamente mandata a NotebookLM, per l'ultima risposta.
    ///
    /// Diventa la faccia della carta di ripasso: "fammi un riassunto del
    /// capitolo 3" e' una domanda, il testo della risposta no.
    pub domanda_notebooklm: String,
    /// Esame scelto per la domanda a NotebookLM.
    ///
    /// Non dipende da Zotero: si puo' chiedere anche a Zotero chiuso, per
    /// qualsiasi esame che abbia una scheda.
    pub esame_notebooklm: Option<String>,
    /// Risposta di NotebookLM riportata dentro l'app, per esame.
    pub risposta_notebooklm: Option<(String, String)>,
    /// Messaggio di conferma mostrato dopo aver salvato un pensiero.
    pub conferma: Option<(String, Instant)>,
    /// Avviso mostrato in giallo (per esempio permessi mancanti).
    pub avviso: Option<String>,
    /// Codice da digitare per lo sblocco di emergenza.
    pub codice_sblocco: Option<String>,
    /// Testo digitato dall'utente per lo sblocco.
    pub testo_sblocco: String,
    /// Numero di caratteri del testo di sblocco al frame precedente.
    pub lunghezza_precedente: usize,
    /// Testo della data d'appello in editing, per esame.
    ///
    /// Deve stare nello stato: un buffer locale verrebbe ricreato a ogni
    /// frame e la data digitata sparirebbe a meta' digitazione.
    pub date_in_edizione: std::collections::BTreeMap<String, String>,
    /// Editor di dominio vietati (una voce per riga).
    pub testo_domini: String,
    /// Editor di parole chiave vietate (una voce per riga).
    pub testo_parole: String,
}

impl StatoApp {
    /// Crea lo stato iniziale e carica la configurazione da disco.
    pub fn nuovo() -> Self {
        let config = carica_configurazione();
        let testo_domini = lista_alle_righe(&config.domini_vietati);
        let testo_parole = lista_alle_righe(&config.parole_vietate);
        let sfide = config.sfide_a_sorpresa;
        let xp = PuntiXp {
            punti: config.xp_punti,
            distrazioni: config.xp_distrazioni,
        };
        Self {
            config,
            sessione: None,
            guardia: None,
            xp,
            xp_a_inizio: (0, 0),
            tempo_documenti: std::collections::BTreeMap::new(),
            ultimo_campione: Instant::now(),
            memoria: Default::default(),
            memoria_esame: String::new(),
            ripasso_indice: None,
            ripasso_risposta_visibile: false,
            ultimo_comando: String::new(),
            anti_sonno: AntiSonno::nuovo(sfide),
            task: crate::task::GestoreTask::nuovo(),
            ai: crate::ai::Assistente::nuovo(),
            storico: crate::storico::carica(),
            ultimo_riepilogo: None,
            collezioni: Vec::new(),
            esame_scelto: None,
            documenti_esame: Vec::new(),
            testi_scelti: Vec::new(),
            interleaving: Interleaving::nuovo(Vec::new()),
            pausa_scade: None,
            evidenziazioni: Vec::new(),
            snapshot_zotero: None,
            campo_notebook: String::new(),
            anteprima_notebooklm: None,
            campo_pensiero: String::new(),
            campo_domanda: String::new(),
            domanda_notebooklm: String::new(),
            esame_notebooklm: None,
            risposta_notebooklm: None,
            conferma: None,
            avviso: None,
            codice_sblocco: None,
            testo_sblocco: String::new(),
            lunghezza_precedente: 0,
            date_in_edizione: std::collections::BTreeMap::new(),
            testo_domini,
            testo_parole,
        }
    }

    /// Vero se c'e' una sessione di studio attiva.
    pub fn in_sessione(&self) -> bool {
        self.sessione.is_some()
    }

    /// Obiettivo mostrato: il testo corrente se c'e' interleaving, altrimenti
    /// l'obiettivo della sessione.
    pub fn obiettivo_corrente(&self) -> String {
        if let Some(testo) = self.interleaving.testo_corrente() {
            if self.interleaving.attivo() {
                return testo.clone();
            }
        }
        self.sessione
            .as_ref()
            .map(|s| s.obiettivo.clone())
            .unwrap_or_else(|| self.config.obiettivo.clone())
    }

    /// Nome dell'esame scelto, per i file e per le domande.
    pub fn nome_esame(&self) -> String {
        self.esame_scelto
            .and_then(|id| self.collezioni.iter().find(|c| c.id == id))
            .map(|c| c.nome.clone())
            .unwrap_or_else(|| "Senza esame".to_string())
    }

    /// Tempo residuo nella sessione corrente.
    pub fn tempo_residuo(&self) -> std::time::Duration {
        self.sessione
            .as_ref()
            .map(|s| s.residuo())
            .unwrap_or(std::time::Duration::ZERO)
    }

    /// Frazione di tempo gia' trascorsa (0.0 - 1.0).
    pub fn frazione_transcorsa(&self) -> f32 {
        self.sessione.as_ref().map(|s| s.frazione()).unwrap_or(0.0)
    }

    /// Vero se la sessione e' momentaneamente in pausa (cambio testo).
    pub fn in_pausa(&self) -> bool {
        self.sessione.as_ref().map(|s| s.in_pausa()).unwrap_or(false)
    }

    /// Aggiunge o toglie un testo dalla selezione, rispettando il massimo.
    pub fn alterna_testo(&mut self, titolo: &str) {
        if let Some(pos) = self.testi_scelti.iter().position(|t| t == titolo) {
            self.testi_scelti.remove(pos);
            return;
        }
        if self.testi_scelti.len() < MAX_TESTI {
            self.testi_scelti.push(titolo.to_string());
        } else {
            self.avviso = Some(format!(
                "Puoi alternare al massimo {MAX_TESTI} testi per sessione."
            ));
        }
    }

    /// Vero se il testo e' fra quelli spuntati.
    pub fn testo_scelto(&self, titolo: &str) -> bool {
        self.testi_scelti.iter().any(|t| t == titolo)
    }

    /// Salva un pensiero improvviso nel file di parcheggio.
    pub fn salva_pensiero(&mut self) {
        match archivi::salva_pensiero(&self.campo_pensiero) {
            Ok(()) => {
                self.campo_pensiero.clear();
                self.aggiungi_notifica("Pensiero salvato al sicuro! Ora torna al testo");
            }
            Err(e) => self.avviso = Some(e),
        }
    }

    /// Mostra una notifica temporanea nella GUI.
    pub fn aggiungi_notifica(&mut self, testo: &str) {
        self.conferma = Some((testo.to_string(), Instant::now()));
    }

    /// Restituisce la notifica temporanea ancora valida (2 secondi).
    pub fn notifica_attiva(&self) -> Option<&str> {
        let (testo, istante) = self.conferma.as_ref()?;
        if istante.elapsed() < std::time::Duration::from_secs(2) {
            Some(testo.as_str())
        } else {
            None
        }
    }

    /// Ricarica la memoria di ripasso dopo aver salvato una carta.
    ///
    /// La carta appena salvata non si vedrebbe fino alla prossima apertura
    /// dell'esame: il pannello mostrava un conteggio fermo.
    pub fn riepilogo_notebooklm(&mut self) {
        self.memoria = crate::carte::carica(&self.nome_esame());
        self.memoria_esame = self.nome_esame();
    }

    /// Salva la configurazione, includendo le liste editate dall'utente.
    pub fn salva_configurazione(&mut self) {
        self.aggiorna_configurazione_da_stato();
        salva_configurazione(&self.config);
    }

    /// Salva la configurazione restituendo un errore se la persistenza fallisce.
    pub fn salva_configurazione_con_esito(&mut self) -> Result<(), String> {
        self.aggiorna_configurazione_da_stato();
        crate::config::salva_configurazione_con_esito(&self.config)
    }

    fn aggiorna_configurazione_da_stato(&mut self) {
        self.config.domini_vietati = righe_alla_lista(&self.testo_domini);
        self.config.parole_vietate = righe_alla_lista(&self.testo_parole);
        // Gli XP fanno parte della configurazione: senza questo si azzerano
        // a ogni riavvio e la gamification perde senso.
        self.config.xp_punti = self.xp.punti;
        self.config.xp_distrazioni = self.xp.distrazioni;
    }

    /// Aggiunge XP e li salva subito, cosi' un crash non li perde.
    pub fn aggiungi_xp(&mut self, punti: i32) {
        self.xp.aggiungi(punti);
        self.salva_configurazione();
    }

    /// Registra una distrazione e salva subito.
    pub fn registra_distrazione(&mut self) {
        self.xp.registra_distrazione();
        self.salva_configurazione();
    }

    /// Allinea la voce del motore audio con l'interruttore in configurazione.
    pub fn sincronizza_voce(&self, audio: &mut MotoreAudio) {
        audio.voce_attiva = self.config.voce_attiva;
    }

    /// Elabora lo stato della voce, trasformandolo in una notifica.
    pub fn aggiorna_voce(&mut self, audio: &mut MotoreAudio) {
        while let Some(evento) = audio.stato_voce() {
            match evento {
                Voce::Errore(m) => self.avviso = Some(m),
                Voce::Interrotta => self.aggiungi_notifica("Lettura interrotta."),
                _ => {}
            }
        }
    }

    /// Modalita' audio attualmente selezionata.
    pub fn modalita_audio(&self) -> ModalitaAudio {
        ModalitaAudio::da_nome(&self.config.modalita_audio)
    }

    /// Imposta la modalita' audio e la salva in configurazione.
    pub fn imposta_modalita_audio(&mut self, modalita: ModalitaAudio) {
        self.config.modalita_audio = modalita.nome().to_string();
    }

    /// Velocita' della voce attualmente selezionata.
    pub fn velocita_voce(&self) -> VelocitaVoce {
        match self.config.velocita_voce.as_str() {
            "Veloce" => VelocitaVoce::Veloce,
            "Turbo" => VelocitaVoce::Turbo,
            _ => VelocitaVoce::Normale,
        }
    }

    /// Imposta la velocita' della voce e la salva in configurazione.
    pub fn imposta_velocita_voce(&mut self, velocita: VelocitaVoce) {
        self.config.velocita_voce = match velocita {
            VelocitaVoce::Normale => "Normale".to_string(),
            VelocitaVoce::Veloce => "Veloce".to_string(),
            VelocitaVoce::Turbo => "Turbo".to_string(),
        };
    }
}

impl Drop for StatoApp {
    /// Fail-safe: alla chiusura il file `hosts` deve tornare sempre pulito.
    fn drop(&mut self) {
        crate::blocker::ripristina_hosts();
        crate::blocker::flush_dns();
    }
}
