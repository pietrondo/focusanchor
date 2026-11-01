//! Pannello di ripasso: le carte scadute oggi, una alla volta, dentro l'app.
//!
//! Prima questo passaggio avveniva in Anki, cioe' in un altro programma: il
//! ciclo studio → dubbio → ripasso si spezzava ogni volta che si cambiava
//! finestra. Qui la carta nasce dalla domanda a tradimento e torna da sola.

use eframe::egui;

use crate::carte::{self, Giudizio};
use crate::state::StatoApp;
use crate::ui::{ACCENTO, BORDO, GIALLO, TENUE, VERDE};

/// Riquadro delle carte di ripasso dell'esame scelto.
pub fn sezione_ripasso(ui: &mut egui::Ui, stato: &mut StatoApp) {
    stato.sincronizza_memoria();
    let oggi = carte::oggi();
    let scadute = stato.memoria.scadute(&oggi);
    let totale = stato.memoria.carte.len();

    if totale == 0 {
        ui.label(
            egui::RichText::new(
                "Nessuna carta. Le carte nascono dalle domande a tradimento: \
                 rispondi a una durante la sessione e la ritrovi qui.",
            )
            .size(13.0)
            .color(TENUE),
        );
        return;
    }

    ui.horizontal(|ui| {
        crate::ui::badge(ui, &format!("{totale} carte"), ACCENTO);
        let colore = if scadute.is_empty() { VERDE } else { GIALLO };
        crate::ui::badge(ui, &format!("{} da ripassare", scadute.len()), colore);
    });

    if scadute.is_empty() {
        ui.label(
            egui::RichText::new("Niente da ripassare oggi: torna domani.")
                .size(13.0)
                .color(TENUE),
        );
        return;
    }

    let Some(indice) = stato.ripasso_indice else {
        if ui
            .add(
                egui::Button::new(egui::RichText::new("🔁 Inizia il ripasso").strong())
                    .min_size(egui::vec2(crate::ui::larghezza(ui), 36.0)),
            )
            .clicked()
        {
            stato.ripasso_indice = stato.memoria.prossima_scaduta(&oggi, None);
            stato.ripasso_risposta_visibile = false;
        }
        return;
    };

    // Si copia subito: il prestito su `memoria` non deve sopravvivere alla
    // chiusura della carta, perche' dopo si modifica lo stato.
    let Some((domanda, risposta)) = stato
        .memoria
        .carte
        .get(indice)
        .map(|carta| (carta.domanda.clone(), carta.risposta.clone()))
    else {
        stato.ripasso_indice = None;
        return;
    };

    egui::Frame::none()
        .fill(egui::Color32::from_rgb(21, 24, 30))
        .stroke(egui::Stroke::new(1.0, BORDO))
        .rounding(egui::Rounding::same(8.0))
        .inner_margin(egui::Margin::same(10.0))
        .show(ui, |ui| {
            ui.set_min_width(crate::ui::larghezza(ui));
            ui.label(
                egui::RichText::new("DOMANDA")
                    .size(10.0)
                    .strong()
                    .color(TENUE),
            );
            ui.label(egui::RichText::new(crate::ui::accorcia(&domanda, 400)).size(14.0));
            ui.add_space(6.0);
            if stato.ripasso_risposta_visibile {
                ui.label(
                    egui::RichText::new("LA TUA RISPOSTA")
                        .size(10.0)
                        .strong()
                        .color(TENUE),
                );
                egui::ScrollArea::vertical()
                    .max_height(160.0)
                    .show(ui, |ui| {
                        ui.label(egui::RichText::new(&risposta).size(14.0).color(ACCENTO));
                    });
            } else if ui.button("👁 Mostra la risposta").clicked() {
                stato.ripasso_risposta_visibile = true;
            }
        });

    if !stato.ripasso_risposta_visibile {
        return;
    }
    // Senza giudizio la carta non si sposta: e' quello che rende utile il giro.
    ui.label(
        egui::RichText::new("Come e' andata?")
            .size(12.0)
            .color(TENUE),
    );
    let meta = ((crate::ui::larghezza(ui) - 10.0) / 3.0).max(80.0);
    ui.horizontal(|ui| {
        for (etichetta, giudizio) in [
            ("❌ Non lo sapevo", Giudizio::Male),
            ("🤔 Così così", Giudizio::Medio),
            ("✅ Lo sapevo", Giudizio::Bene),
        ] {
            if ui
                .add(egui::Button::new(etichetta).min_size(egui::vec2(meta, 34.0)))
                .clicked()
            {
                giudica(stato, indice, giudizio, &oggi);
            }
        }
    });
}

/// Registra il giudizio, salva e passa alla carta successiva.
fn giudica(stato: &mut StatoApp, indice: usize, giudizio: Giudizio, oggi: &str) {
    let giorno = chrono::Local::now().date_naive();
    if let Some(carta) = stato.memoria.carte.get_mut(indice) {
        carte::ripassa(carta, giudizio, giorno);
    }
    if let Err(e) = carte::salva(&stato.memoria_esame.clone(), &stato.memoria) {
        stato.avviso = Some(e);
    }
    stato.ripasso_risposta_visibile = false;
    stato.ripasso_indice = stato.memoria.prossima_scaduta(oggi, Some(indice));
}
