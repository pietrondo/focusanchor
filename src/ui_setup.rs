//! Modalita' Configurazione: esame, testi, durata e audio.

use std::sync::mpsc::Sender;

use eframe::egui;

use crate::audio::MotoreAudio;
use crate::blocker::Segnale;
use crate::state::{StatoApp, DURATA_MAXIMA};
use crate::ui::{avviso, barra_xp, notifica, scheda, ACCENTO, ARANCIO, TENUE, VERDE};

/// Disegna la modalita' Configurazione (prima della sessione).
pub fn disegna(
    ctx: &egui::Context,
    stato: &mut StatoApp,
    audio: &mut MotoreAudio,
    mittente: &Sender<Segnale>,
) {
    egui::CentralPanel::default()
        .frame(
            egui::Frame::none()
                .fill(crate::ui::SFONDO)
                .inner_margin(egui::Margin::symmetric(18.0, 12.0)),
        )
        .show(ctx, |ui| {
            ui.add_space(2.0);
            intestazione(ui, stato);
            ui.add_space(10.0);

            // La larghezza si calcola qui fuori: dentro lo ScrollArea
            // potrebbe essere infinita.
            let l = crate::ui::larghezza_schede(ui);
            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    let disponibili = !stato.collezioni.is_empty();
                    scheda(ui, l, "🎯", "Obiettivo", ACCENTO, |ui| {
                        sezione_obiettivo(ui, stato, disponibili);
                    });

                    scheda(ui, l, "📚", "Esame e testi", VERDE, |ui| {
                        crate::ui_zotero::sezione_esame(ui, stato);
                    });

                    scheda(ui, l, "⏱", "Durata", ACCENTO, |ui| {
                        sezione_durata(ui, stato);
                    });

                    scheda(ui, l, "🎧", "Suono", ARANCIO, |ui| {
                        crate::ui_audio::sezione_audio(ui, stato, audio);
                    });

                    scheda(ui, l, "📂", "I tuoi file", TENUE, |ui| {
                        crate::ui_archivi::sezione_archivi(ui, stato);
                    });

                    scheda(ui, l, "📈", "Storico", VERDE, |ui| {
                        crate::ui_storico::sezione_storico(ui, stato);
                    });

                    ui.add_space(4.0);
                    pulsante_avvia(ui, stato, audio, mittente);

                    notifica(ui, stato.notifica_attiva());
                    avviso(ui, &stato.avviso);
                    ui.add_space(10.0);
                });
        });
}

/// Intestazione con nome dell'app e stato dei punti.
fn intestazione(ui: &mut egui::Ui, stato: &StatoApp) {
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new("🎯").size(30.0));
        ui.vertical(|ui| {
            ui.label(
                egui::RichText::new("FocusAnchor")
                    .size(25.0)
                    .strong()
                    .color(crate::ui::TESTO),
            );
            ui.label(
                egui::RichText::new("Un obiettivo alla volta. Cinque esami. Dicembre.")
                    .size(13.0)
                    .color(TENUE),
            );
        });
    });
    ui.add_space(8.0);
    barra_xp(ui, stato);
}

/// Campo di testo con l'obiettivo, oppure la selezione dei testi.
fn sezione_obiettivo(ui: &mut egui::Ui, stato: &mut StatoApp, con_esame: bool) {
    if con_esame {
        // Con un esame scelto l'obiettivo nasce dai testi spuntati.
        let corrente = stato.obiettivo_corrente();
        ui.label(
            egui::RichText::new(if corrente.trim().is_empty() {
                "Scegli i testi qui sotto: l'obiettivo si compila da solo.".to_string()
            } else {
                corrente
            })
            .size(16.0)
            .strong(),
        );
        ui.label(
            egui::RichText::new(
                "Puoi comunque scrivere un obiettivo tuo qui sotto.",
            )
            .size(12.0)
            .color(TENUE),
        );
    }
    egui::TextEdit::singleline(&mut stato.config.obiettivo)
        .hint_text("Esempio: Cap. 3 Storia Romana - Zotero")
        .desired_width(f32::INFINITY)
        .font(egui::TextStyle::Body)
        .show(ui);
    // Niente trim mentre si scrive: togliere gli spazi renderebbe impossibile
    // digitare "Cap. 3 Storia Romana".
}

/// Scelta rapida della durata della sessione.
fn sezione_durata(ui: &mut egui::Ui, stato: &mut StatoApp) {
    ui.horizontal(|ui| {
        for minuti in [25u32, 45, 60] {
            let selezionato = stato.config.durata_minuti == minuti;
            let testo = format!("{minuti} min");
            if ui.selectable_label(selezionato, testo).clicked() {
                stato.config.durata_minuti = minuti;
            }
        }
    });
    ui.add_space(4.0);
    let mut durata = stato.config.durata_minuti;
    // Lo slider ha una larghezza minima che a finestre strette sfonda la
    // scheda: la si adatta allo spazio reale.
    ui.spacing_mut().slider_width = (crate::ui::larghezza(ui) * 0.45).clamp(50.0, 200.0);
    if ui
        .add(egui::Slider::new(&mut durata, 5..=DURATA_MAXIMA).suffix(" min"))
        .changed()
    {
        stato.config.durata_minuti = durata;
    }
}

/// Pulsante di avvio: applica il blocco hosts e apre la sessione.
fn pulsante_avvia(
    ui: &mut egui::Ui,
    stato: &mut StatoApp,
    audio: &mut MotoreAudio,
    mittente: &Sender<Segnale>,
) {
    let pronto = !stato.config.obiettivo.trim().is_empty();
    let colore = if pronto { VERDE } else { crate::ui::BORDO };
    let pulsante = egui::Button::new(
        egui::RichText::new("▶  INIZIA LA SESSIONE")
            .size(18.0)
            .strong()
            .color(if pronto { crate::ui::SFONDO } else { TENUE }),
    )
    .fill(colore)
    .min_size(egui::vec2(crate::ui::larghezza(ui), 50.0));
    let risposta = ui.add_enabled(pronto, pulsante);
    if !pronto {
        ui.label(
            egui::RichText::new("Scrivi l'obiettivo o scegli un testo da Zotero.")
                .size(13.0)
                .color(crate::ui::GIALLO),
        );
        return;
    }
    if risposta.clicked() {
        crate::ui_esami::avvia_sessione(stato, audio, mittente);
    }
}
