//! Scheda di un esame: link al corso, crediti, modalita' e data d'appello.

use eframe::egui;

use crate::state::StatoApp;
use crate::ui::TENUE;

/// Scheda dell'esame: link al corso, crediti, modalita' e data d'appello.
///
/// L'esame arriva dall'esterno: la scheda deve funzionare anche quando
/// Zotero non e' caricato, quindi non puo' dipendere dalla collezione
/// selezionata.
pub fn sezione_scheda_esame(ui: &mut egui::Ui, stato: &mut StatoApp, esame: &str) {
    if esame.trim().is_empty() {
        return;
    }
    let esame = esame.to_string();

    let mut info = stato.config.esami.get(&esame).cloned().unwrap_or_default();
    let mut salva = false;
    let mut apri = false;

    egui::CollapsingHeader::new("📋 Scheda dell'esame")
        .default_open(info.data_esame.is_empty())
        .show(ui, |ui| {
            ui.label(
                egui::RichText::new("Pagina del corso")
                    .size(12.0)
                    .color(TENUE),
            );
            egui::TextEdit::singleline(&mut info.url)
                .hint_text("https://www.unimi.it/…")
                .desired_width(f32::INFINITY)
                .show(ui);

            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("Crediti").size(12.0).color(TENUE));
                ui.add(egui::DragValue::new(&mut info.crediti).range(0..=30));
                ui.checkbox(&mut info.non_frequentante, "Non frequentante");
            });

            ui.horizontal_wrapped(|ui| {
                ui.label(egui::RichText::new("Modalità").size(12.0).color(TENUE));
                for modalita in ["Scritto", "Orale", "Scritto e orale"] {
                    let scelto = info.modalita == modalita;
                    if ui.selectable_label(scelto, modalita).clicked() {
                        info.modalita = modalita.to_string();
                    }
                }
            });

            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("Appello").size(12.0).color(TENUE));
                crate::ui_appelli::campo_data(ui, stato, &esame);
                // Il conto alla rovescia accanto al campo: qui si guarda un
                // esame alla volta, e serve sapere quanto manca.
                let data = stato
                    .config
                    .esami
                    .get(&esame)
                    .map(|info| info.data_esame.clone())
                    .unwrap_or_default();
                if let Some(giorni) = crate::storico::giorni_alla_data(&data) {
                    let (testo, colore) = match giorni {
                        g if g < 0 => ("passato".to_string(), crate::ui::TENUE),
                        0 => ("oggi".to_string(), crate::ui::GIALLO),
                        g => (format!("mancano {g} giorni"), crate::ui::ARANCIO),
                    };
                    ui.label(egui::RichText::new(testo).size(12.0).color(colore));
                }
            });

            ui.horizontal(|ui| {
                if ui.button("💾 Salva scheda").clicked() {
                    salva = true;
                }
                if !info.url.trim().is_empty() && ui.button("▶ Apri la pagina").clicked() {
                    apri = true;
                }
            });
        });

    applica_azioni(stato, esame, info, salva, apri);
}

/// Salva la scheda o apre la pagina del corso, secondo i pulsanti premuti.
fn applica_azioni(
    stato: &mut StatoApp,
    esame: String,
    info: crate::config::InfoEsame,
    salva: bool,
    apri: bool,
) {
    if salva {
        let mut da_salvare = info.clone();
        da_salvare.url = da_salvare.url.trim().to_string();
        // La data la scrive `ui_appelli::campo_data` appena si digita: qui si
        // riprende quella, non la copia di inizio frame che sarebbe vecchia.
        da_salvare.data_esame = stato
            .config
            .esami
            .get(&esame)
            .map(|salvata| salvata.data_esame.clone())
            .unwrap_or_default();
        stato.config.esami.insert(esame, da_salvare);
        stato.salva_configurazione();
        stato.aggiungi_notifica("Scheda dell'esame salvata.");
    }
    if apri {
        if let Err(e) = crate::notebooklm::apri(&info.url) {
            stato.avviso = Some(e);
        }
    }
}
