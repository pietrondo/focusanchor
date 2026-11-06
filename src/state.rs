//! Stato dell'applicazione: esame scelto, testi, XP e archivio.

use std::path::PathBuf;
use std::time::Instant;

use crate::anti_sleep::AntiSonno;
use crate::archivi;
use crate::audio::{ModalitaAudio, MotoreAudio, Voce};
use crate::audio_voce::VelocitaVoce;
use crate::config::{
    carica_configurazione, lista_alle_righe, righe_alla_lista, Configurazione,
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
    /// Snapshot Zotero in arrivo dal thread che lo prepara, con il lavoro
    /// da fare quando arriva.
    pub snapshot_in_arrivo: Option<(
        std::sync::mpsc::Receiver<Result<PathBuf, String>>,
        crate::ui_esami::LavoroZotero,
    )>,
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
    /// Finestra di Zotero agganciata dentro la nostra, se c'e'.
    ///
    /// Conserva l'handle per poterla staccare e ripristinare: una finestra
    /// agganciata che resta orfana sparisce dallo schermo.
    pub zotero_incassato: Option<isize>,
    /// Scadenza dell'attesa della finestra di Zotero.
    ///
    /// L'attesa non blocca: si prova a ogni frame finche' la finestra non
    /// compare, e intanto l'interfaccia resta viva.
    pub attesa_zotero: Option<Instant>,
    /// Riquadro dove disegnare Zotero, in punti egui.
    pub area_zotero: Option<egui::Rect>,
    /// Pagina di NotebookLM incorporata, se aperta.
    pub pagina: Option<crate::webview::Pagina>,
    /// Riquadro dove disegnare la pagina, in punti egui.
    pub area_pagina: Option<egui::Rect>,
    /// Vero se il campo per incollare il link del notebook e' aperto.
    pub mostra_campo_link: bool,
    /// Domande a NotebookLM e loro risposte.
    pub chiedi: crate::notebooklm::Chiedi,
    /// Generazione di flashcard e lettura delle fonti.
    pub genera: crate::notebooklm::Genera,
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
    /// Editor di parole chiave vietate (una voce per riga).
    pub testo_parole: String,
    /// Secondi passati su ogni documento, raggruppati per esame.
    ///
    /// Si calcolano quando lo storico cambia, non a ogni frame: rifarli
    /// clona il titolo di ogni documento di ogni sessione.
    pub secondi_documenti: std::collections::BTreeMap<String, std::collections::BTreeMap<String, u32>>,
    /// Ritmo recente dell'esame in studio, in minuti al giorno.
    pub piano_ritmo: u32,
    /// Minuti al giorno che servono per arrivare all'appello.
    pub piano_minuti_al_giorno: Option<u32>,
}

impl StatoApp {
    /// Crea lo stato iniziale e carica la configurazione da disco.
    pub fn nuovo() -> Self {
        let config = carica_configurazione();
        let testo_parole = lista_alle_righe(&config.parole_vietate);
        let sfide = config.sfide_a_sorpresa;
        let xp = PuntiXp {
            punti: config.xp_punti,
            distrazioni: config.xp_distrazioni,
        };
        let storico = crate::storico::carica();
        let secondi_documenti = crate::appelli::secondi_per_documento(&storico);
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
            storico,
            ultimo_riepilogo: None,
            collezioni: Vec::new(),
            esame_scelto: None,
            documenti_esame: Vec::new(),
            testi_scelti: Vec::new(),
            interleaving: Interleaving::nuovo(Vec::new()),
            pausa_scade: None,
            evidenziazioni: Vec::new(),
            snapshot_zotero: None,
            snapshot_in_arrivo: None,
            campo_notebook: String::new(),
            anteprima_notebooklm: None,
            campo_pensiero: String::new(),
            campo_domanda: String::new(),
            domanda_notebooklm: String::new(),
            esame_notebooklm: None,
            zotero_incassato: None,
            attesa_zotero: None,
            area_zotero: None,
            pagina: None,
            area_pagina: None,
            mostra_campo_link: false,
            chiedi: crate::notebooklm::Chiedi::nuovo(),
            genera: crate::notebooklm::Genera::nuovo(),
            conferma: None,
            avviso: None,
            codice_sblocco: None,
            testo_sblocco: String::new(),
            lunghezza_precedente: 0,
            date_in_edizione: std::collections::BTreeMap::new(),
            testo_parole,
            secondi_documenti,
            piano_ritmo: 0,
            piano_minuti_al_giorno: None,
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

    /// Salva la configurazione, dicendo se non e' riuscita.
    ///
    /// Non si puo' zitticare l'errore: se la cartella non e' scrivibile,
    /// l'XP e le impostazioni andrebbero persi senza che l'utente lo sappia.
    pub fn salva_configurazione(&mut self) {
        if let Err(e) = self.salva_configurazione_con_esito() {
            self.avviso = Some(format!("Configurazione non salvata: {e}"));
        }
    }

    /// Salva la configurazione restituendo un errore se la persistenza fallisce.
    pub fn salva_configurazione_con_esito(&mut self) -> Result<(), String> {
        self.aggiorna_configurazione_da_stato();
        crate::config::salva_configurazione_con_esito(&self.config)
    }

    fn aggiorna_configurazione_da_stato(&mut self) {
        self.config.parole_vietate = righe_alla_lista(&self.testo_parole);
        // Gli XP fanno parte della configurazione: senza questo si azzerano
        // a ogni riavvio e la gamification perde senso.
        self.config.xp_punti = self.xp.punti;
        self.config.xp_distrazioni = self.xp.distrazioni;
    }

    /// Ricalcola il piano di studio e il ritmo dell'esame corrente.
    ///
    /// Va fatto quando lo storico o l'esame cambiano, non a ogni frame:
    /// il calcolo clona il titolo di ogni documento e parsa ogni data.
    pub fn ricalcola_piano(&mut self) {
        let esame = self.nome_esame();
        let ritmo = crate::appelli::ritmo_esame(&self.storico, &esame);
        let giorni = crate::appelli::conto_di(&self.config, &esame).map(|(giorni, _)| giorni);
        let piano = crate::appelli::piano(
            &esame,
            &self.secondi_documenti,
            &self.collezioni,
            giorni,
        );
        self.piano_ritmo = ritmo;
        self.piano_minuti_al_giorno = piano.minuti_al_giorno;
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
