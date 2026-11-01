//! FocusAnchor - ancoraggio anti-distrazione per chi studia con ADHD e dislessia.
//!
//! Entry point: pulizia dei residui del file `hosts`, panic hook di sicurezza,
//! avvio della finestra `eframe`.

mod ai;
mod anti_sleep;
mod archivi;
mod audio;
mod audio_voce;
mod blocker;
mod config;
mod finestre;
mod guardia;
mod interleaving;
mod notebooklm;
mod sessione;
mod state;
mod storico;
mod suono;
mod task;
mod tts;
mod ui;
mod ui_ai;
mod ui_archivi;
mod ui_audio;
mod ui_azioni;
mod ui_caricamento;
mod ui_esami;
mod ui_focus;
mod ui_sblocco;
mod ui_scheda;
mod ui_setup;
mod ui_storico;
mod ui_sfida;
mod ui_zotero;
mod xp;
mod zotero;
mod zotero_query;

#[cfg(test)]
mod test_interfaccia;

use std::sync::mpsc::{Receiver, Sender};

use eframe::egui;

use crate::blocker::Segnale;
use crate::state::StatoApp;

/// Dimensione dell'overlay compatto durante la sessione.
const DIMENSIONE_OVERLAY: [f32; 2] = [420.0, 380.0];

/// Dimensione della finestra principale in modalita' Configurazione.
const DIMENSIONE_CONFIGURAZIONE: [f32; 2] = [620.0, 860.0];

/// Struttura principale dell'applicazione.
struct AppFocus {
    stato: StatoApp,
    audio: audio::MotoreAudio,
    canale: Receiver<Segnale>,
    mittente: Sender<Segnale>,
    /// Ricorda se la finestra e' gia' in modalita' overlay.
    overlay_attivo: bool,
}

impl AppFocus {
    fn nuovo(cc: &eframe::CreationContext<'_>) -> Self {
        ui::applica_tema(&cc.egui_ctx);
        let (mittente, canale) = std::sync::mpsc::channel();
        Self {
            stato: StatoApp::nuovo(),
            audio: audio::MotoreAudio::nuovo(),
            canale,
            mittente,
            overlay_attivo: false,
        }
    }

    /// Elabora gli eventi arrivati dalla guardia finestre.
    fn gestisci_segnali(&mut self) {
        while let Ok(segnale) = self.canale.try_recv() {
            match segnale {
                Segnale::Distrazione(titolo) => self.annuncia_distrazione(&titolo),
                Segnale::ContestoZotero(documento) => self.annuncia_contesto(&documento),
            }
        }
    }

    /// Avvisa che su Zotero e' aperto un PDF di un altro esame.
    ///
    /// Non toglie XP: cambiare testo puo' essere legittimo. L'avviso serve
    /// solo a riportare l'attenzione, e il cooldown evita che diventi fastidio.
    fn annuncia_contesto(&mut self, documento: &str) {
        let esame = self.stato.nome_esame();
        self.audio.parla(
            "Attenzione al cambio di contesto! Stai studiando un altro esame, \
             torna sul testo corrente!",
        );
        self.stato.aggiungi_notifica(&format!(
            "«{documento}» non e' dell'esame {esame}."
        ));
    }

    /// Avvisa l'utente con la voce che ha lasciato perdere il filo del discorso.
    ///
    /// La distrazione costa anche dei punti XP: il rinforzo negativo e' il
    /// segnale che la distrazione ha un costo, non solo una nota.
    fn annuncia_distrazione(&mut self, titolo: &str) {
        self.stato.registra_distrazione();
        let obiettivo = self.stato.obiettivo_corrente().to_string();
        let frase = format!(
            "Fermo! Stai studiando {obiettivo}. Chiudi le distrazioni e torna su Zotero!"
        );
        self.audio.parla(&frase);
        self.stato.aggiungi_notifica(&format!(
            "Distrazione sventata: {titolo}. Torna sull'obiettivo."
        ));
    }

    /// Controlla se e' ora di porre una domanda a tradimento.
    fn controlla_sfida(&mut self) {
        if !self.stato.anti_sonno.domanda_da_porre() {
            return;
        }
        // Si usa una sottolineatura recente di Zotero, se disponibile.
        let passaggio = self
            .stato
            .evidenziazioni
            .first()
            .map(|e| e.testo.clone())
            .filter(|t| !t.trim().is_empty());
        self.stato.anti_sonno.imposta_passaggio(passaggio);

        let domanda = match self.stato.anti_sonno.avvia_domanda() {
            Some(passaggio) => format!(
                "Sfida lampo! Hai sottolineato questo passaggio su Zotero: {passaggio}. \
                 Riassumi in una frase cosa hai appena letto senza guardare il PDF!"
            ),
            None => {
                "Sfida lampo! Riassumi in una frase cosa hai appena letto senza guardare il PDF!"
                    .to_string()
            }
        };
        self.audio.beep();
        self.audio.parla(&domanda);
    }

    /// Adatta la finestra alla modalita' corrente (overlay o configurazione).
    fn aggiorna_vista(&mut self, ctx: &egui::Context) {
        let in_sessione = self.stato.in_sessione();
        if in_sessione == self.overlay_attivo {
            return;
        }
        self.overlay_attivo = in_sessione;
        if in_sessione {
            ctx.send_viewport_cmd(egui::ViewportCommand::WindowLevel(
                egui::WindowLevel::AlwaysOnTop,
            ));
            ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(egui::vec2(
                DIMENSIONE_OVERLAY[0],
                DIMENSIONE_OVERLAY[1],
            )));
            // L'overlay va nell'angolo in basso a destra: in alto coprirebbe
            // le schede e la barra degli strumenti di Zotero.
            if let Some(posizione) = posizione_angolo(ctx, DIMENSIONE_OVERLAY) {
                ctx.send_viewport_cmd(egui::ViewportCommand::OuterPosition(posizione));
            }
        } else {
            ctx.send_viewport_cmd(egui::ViewportCommand::WindowLevel(
                egui::WindowLevel::Normal,
            ));
            ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(egui::vec2(
                DIMENSIONE_CONFIGURAZIONE[0],
                DIMENSIONE_CONFIGURAZIONE[1],
            )));
        }
    }
}

/// Calcola la posizione dell'angolo in basso a destra dello schermo.
///
/// `GetSystemMetrics` restituisce pixel fisici mentre egui lavora in punti:
/// la divisione per `pixels_per_point` tiene conto dello scaling di Windows.
fn posizione_angolo(ctx: &egui::Context, dimensione: [f32; 2]) -> Option<egui::Pos2> {
    // Area utile: cosi' la finestra non finisce sotto la barra delle
    // applicazioni.
    let (larghezza, altezza) = crate::finestre::area_utile();
    let scala = ctx.pixels_per_point();
    if !scala.is_finite() || scala <= 0.0 {
        return None;
    }
    let x = (larghezza / scala - dimensione[0] - 24.0).max(0.0);
    // La posizione e' dell'angolo esterno, che comprende la barra del titolo:
    // senza questo margine la finestra scivola sotto la barra delle applicazioni.
    const BARRA_TITOLO: f32 = 40.0;
    let y = (altezza / scala - dimensione[1] - BARRA_TITOLO - 16.0).max(0.0);
    Some(egui::pos2(x, y))
}

impl eframe::App for AppFocus {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.gestisci_segnali();
        self.stato.sincronizza_voce(&mut self.audio);
        self.stato.ai.aggiorna();
        self.stato.aggiorna_sessione(&mut self.audio);
        self.stato.aggiorna_interleaving(&mut self.audio);
        self.stato.aggiorna_voce(&mut self.audio);
        self.controlla_sfida();

        if self.stato.in_sessione() {
            ui_focus::disegna(ctx, &mut self.stato, &mut self.audio);
        } else {
            ui_setup::disegna(ctx, &mut self.stato, &mut self.audio, &self.mittente);
        }
        self.aggiorna_vista(ctx);
        ctx.request_repaint();
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        // `termina_sessione` ripristina gia' il file `hosts` e ferma la guardia.
        self.stato.termina_sessione();
        self.stato.salva_configurazione();
        self.audio.ferma_voce();
        self.audio.ferma_sottofondo();
    }
}

fn main() {
    // Fail-safe: un crash precedente puo' aver lasciato il file `hosts` sporco.
    blocker::ripristina_hosts();
    blocker::flush_dns();

    // Fail-safe: durante un panic il file `hosts` deve tornare pulito comunque.
    let hook_precedente = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        blocker::ripristina_hosts();
        blocker::flush_dns();
        hook_precedente(info);
    }));

    let opzioni = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size(DIMENSIONE_CONFIGURAZIONE)
            .with_min_inner_size([560.0, 560.0])
            .with_title("FocusAnchor"),
        ..Default::default()
    };

    let risultato = eframe::run_native(
        "FocusAnchor",
        opzioni,
        Box::new(|cc| Ok(Box::new(AppFocus::nuovo(cc)))),
    );

    if let Err(errore) = risultato {
        eprintln!("FocusAnchor non e' riuscito ad avviarsi: {errore}");
    }
}
