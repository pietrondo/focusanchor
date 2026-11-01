//! Caricamento degli esami da Zotero: pulsanti e schermata vuota.

use eframe::egui;

use crate::state::StatoApp;
use crate::ui::TENUE;

/// Stato vuoto: nessun esame caricato, con le due azioni in evidenza.
pub fn stato_vuoto(ui: &mut egui::Ui, stato: &mut StatoApp) {
    egui::Frame::none()
        .fill(egui::Color32::from_rgb(21, 24, 30))
        .stroke(egui::Stroke::new(1.0, crate::ui::BORDO))
        .rounding(egui::Rounding::same(10.0))
        .inner_margin(egui::Margin::same(16.0))
        .show(ui, |ui| {
            let l = crate::ui::larghezza(ui);
            ui.set_min_width(l);
            ui.vertical_centered(|ui| {
                ui.add_space(4.0);
                ui.label(egui::RichText::new("📚").size(26.0));
                ui.label(
                    egui::RichText::new("Nessun esame caricato")
                        .size(15.0)
                        .strong(),
                );
                ui.label(
                    egui::RichText::new(
                        "Apri Zotero, poi carica le collezioni: ognuna diventa un esame.",
                    )
                    .size(12.0)
                    .color(TENUE),
                );
                ui.add_space(10.0);
                if ui
                    .add(
                        egui::Button::new(
                            egui::RichText::new("🔄 Carica esami da Zotero")
                                .size(15.0)
                                .strong(),
                        )
                        .fill(crate::ui::VERDE)
                        .min_size(egui::vec2(l.min(320.0), 40.0)),
                    )
                    .clicked()
                {
                    crate::ui_esami::carica_esami(stato);
                }
                ui.add_space(4.0);
                if ui
                    .add(
                        egui::Button::new("🔍 Rileva PDF aperto")
                            .min_size(egui::vec2(l.min(320.0), 30.0)),
                    )
                    .clicked()
                {
                    crate::ui_esami::rileva_pdf_zotero(stato);
                }
                ui.add_space(4.0);
            });
        });
}

/// Pulsanti di caricamento, a larghezza uguale.
pub fn pulsanti_caricamento(ui: &mut egui::Ui, stato: &mut StatoApp) {
    let l = crate::ui::larghezza(ui);
    let meta = ((l - 10.0) / 2.0).max(90.0);
    ui.horizontal(|ui| {
        if ui
            .add(egui::Button::new("🔄 Carica da Zotero").min_size(egui::vec2(meta, 32.0)))
            .clicked()
        {
            crate::ui_esami::carica_esami(stato);
        }
        if ui
            .add(egui::Button::new("🔍 Rileva PDF").min_size(egui::vec2(meta, 32.0)))
            .clicked()
        {
            crate::ui_esami::rileva_pdf_zotero(stato);
        }
    });
}
