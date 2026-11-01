//! Riquadro dello storico: cosa hai fatto, quanto, su quale esame.

use eframe::egui;

use crate::state::StatoApp;
use crate::storico;
use crate::ui::{ACCENTO, TENUE, VERDE, VIOLA};

/// Riepilogo delle sessioni concluse.
pub fn sezione_storico(ui: &mut egui::Ui, stato: &mut StatoApp) {
    let stat = storico::statistiche(&stato.storico);
    if stat.sessioni == 0 {
        ui.label(
            egui::RichText::new(
                "Nessuna sessione conclusa. Il primo giro di studio comparirà qui.",
            )
            .size(13.0)
            .color(TENUE),
        );
        return;
    }

    ui.horizontal(|ui| {
        crate::ui::badge(ui, &format!("{} sessioni", stat.sessioni), ACCENTO);
        crate::ui::badge(ui, &format!("{} h {} min", stat.minuti / 60, stat.minuti % 60), VERDE);
        crate::ui::badge(ui, &format!("{} XP", stat.xp), VIOLA);
    });
    ui.label(
        egui::RichText::new(format!(
            "Ultimi 7 giorni: {} minuti · distrazioni sventate: {}",
            stat.ultimi_7_giorni, stat.distrazioni
        ))
        .size(12.0)
        .color(TENUE),
    );

    if !stat.per_esame.is_empty() {
        ui.add_space(4.0);
        ui.label(
            egui::RichText::new("Tempo per esame")
                .size(12.0)
                .strong()
                .color(TENUE),
        );
        let massimo = stat.per_esame.iter().map(|(_, m)| *m).max().unwrap_or(1).max(1);
        for (esame, minuti) in stat.per_esame.iter().take(6) {
            // Barra proporzionale: si vede a colpo d'occhio dove sei indietro.
            let frazione = *minuti as f32 / massimo as f32;
            let etichetta = format!("{}  {} min", crate::ui::accorcia(esame, 34), minuti);
            ui.add(
                egui::ProgressBar::new(frazione)
                    .desired_height(16.0)
                    .rounding(egui::Rounding::same(6.0))
                    .fill(ACCENTO.gamma_multiply(0.7))
                    .text(egui::RichText::new(etichetta).size(11.0)),
            );
        }
    }
}

/// Giorni che mancano all'appello piu' vicino, per l'intestazione.
pub fn prossimo_esame(stato: &StatoApp) -> Option<(String, i64)> {
    stato
        .config
        .esami
        .iter()
        .filter_map(|(esame, info)| {
            let giorni = storico::giorni_alla_data(&info.data_esame)?;
            if giorni < 0 {
                return None;
            }
            Some((esame.clone(), giorni))
        })
        .min_by_key(|(_, giorni)| *giorni)
}
