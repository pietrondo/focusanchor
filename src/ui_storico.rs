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

/// Appello piu' vicino: quanti giorni mancano e quali esami cadono quel giorno.
///
/// Piu' esami sullo stesso appello sono la norma a dicembre: mostrare solo il
/// primo nome (spesso il piu' lungo di Zotero) nasconderebbe gli altri.
pub fn prossimo_esame(stato: &StatoApp) -> Option<(i64, Vec<String>)> {
    let mut giorni_min: Option<i64> = None;
    let mut nomi: Vec<String> = Vec::new();
    for (esame, info) in &stato.config.esami {
        let Some(giorni) = storico::giorni_alla_data(&info.data_esame) else {
            continue;
        };
        if giorni < 0 {
            continue;
        }
        match giorni_min {
            None => {
                giorni_min = Some(giorni);
                nomi.push(esame.clone());
            }
            Some(minimo) if giorni < minimo => {
                giorni_min = Some(giorni);
                nomi.clear();
                nomi.push(esame.clone());
            }
            Some(minimo) if giorni == minimo => nomi.push(esame.clone()),
            _ => {}
        }
    }
    giorni_min.map(|giorni| (giorni, nomi))
}

/// Riga del conto alla rovescia per l'intestazione della sessione.
pub fn testo_conto(giorni: i64, esami: &[String]) -> String {
    let quando = match giorni {
        0 => "oggi".to_string(),
        1 => "domani".to_string(),
        n => format!("fra {n} giorni"),
    };
    match esami {
        [] => format!("📅 Appello {quando}"),
        [unico] => format!("📅 {}: {quando}", crate::ui::accorcia(unico, 30)),
        molti => format!("📅 {} esami {quando}", molti.len()),
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::config::InfoEsame;

    fn esame(data: &str) -> InfoEsame {
        InfoEsame {
            data_esame: data.to_string(),
            ..Default::default()
        }
    }

    #[test]
    fn piu_esami_lo_stesso_giorno_contano_insieme() {
        let data = (chrono::Local::now().date_naive() + chrono::Duration::days(44))
            .format("%Y-%m-%d")
            .to_string();
        let mut stato = StatoApp::nuovo();
        stato.config.esami.clear();
        stato.config.esami.insert("Storia romana".into(), esame(&data));
        stato.config.esami.insert("Letteratura".into(), esame(&data));
        // Un appello gia' passato non deve diventare "il piu' vicino".
        stato.config.esami.insert("Vecchio".into(), esame("2020-01-01"));

        let (giorni, nomi) = prossimo_esame(&stato).expect("appello");
        assert_eq!(giorni, 44);
        assert_eq!(nomi.len(), 2);
    }

    #[test]
    fn la_riga_del_conto_alla_rovescia_e_leggibile() {
        assert!(testo_conto(0, &[]).contains("oggi"));
        assert!(testo_conto(1, &["Storia".into()]).contains("domani"));
        // Con piu' esami si conta, senza tirarsi dietro il nome piu' lungo.
        let tre: Vec<String> = ["A", "B", "C"].iter().map(|s| s.to_string()).collect();
        assert_eq!(testo_conto(44, &tre), "📅 3 esami fra 44 giorni");
        assert_eq!(testo_conto(44, &["Storia romana".into()]), "📅 Storia romana: fra 44 giorni");
    }
}
