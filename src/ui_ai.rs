//! Pannello con la risposta dell'assistente AI.

use eframe::egui;

use crate::state::StatoApp;
use crate::ui::{ACCENTO, SFONDO, TENUE};

/// Mostra la risposta dell'assistente AI, se c'e'.
pub fn mostra_risposta(ui: &mut egui::Ui, stato: &mut StatoApp) {
    if stato.ai.in_corso {
        ui.horizontal(|ui| {
            ui.spinner();
            ui.label(
                egui::RichText::new("L'assistente sta leggendo il passaggio…")
                    .size(13.0)
                    .color(TENUE),
            );
        });
        return;
    }
    let Some(risposta) = stato.ai.risposta.clone() else {
        return;
    };

    egui::Frame::none()
        .fill(SFONDO)
        .stroke(egui::Stroke::new(1.0, ACCENTO.gamma_multiply(0.5)))
        .rounding(egui::Rounding::same(8.0))
        .inner_margin(egui::Margin::same(10.0))
        .show(ui, |ui| {
            let larghezza = crate::ui::larghezza(ui);
            ui.set_min_width(larghezza);
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new("🤖 Spiegazione")
                        .size(13.0)
                        .strong()
                        .color(ACCENTO),
                );
                if ui.small_button("✕").clicked() {
                    stato.ai.pulisci();
                }
            });
            egui::ScrollArea::vertical()
                .max_height(180.0)
                .show(ui, |ui| {
                    ui.label(egui::RichText::new(&risposta).size(14.0));
                });
        });
}
