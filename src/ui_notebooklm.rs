//! Chiedere a NotebookLM e ricevere la risposta dentro l'app.
//!
//! Il flusso naturale e': scrivi la domanda, la mandi a NotebookLM, e quando
//! arriva la spiegazione la riporti qui perche' diventi materiale di studio.
//!
//! Il punto di attrito era che la risposta viveva solo nel browser: si
//! rileggeva, si ricopiava a mano, e finiva nel vuoto. Qui la risposta
//! torna nell'app e puo' diventare una carta di ripasso.
//!
//! Non serve Zotero: gli esami sono quelli con una scheda in `config.esami`,
//! quindi si puo' chiedere anche con Zotero chiuso.

use eframe::egui;

use crate::state::StatoApp;
use crate::ui::{badge, ACCENTO, TENUE, VERDE};

/// Lunghezza massima della domanda che diventa la faccia di una carta.
const MASSIMO_DOMANDA: usize = 240;

/// Pannello "Chiedi a NotebookLM": domanda, invio e risposta.
pub fn sezione_chiedi(ui: &mut egui::Ui, stato: &mut StatoApp) {
    let esami: Vec<String> = stato.config.esami.keys().cloned().collect();
    if esami.is_empty() {
        ui.label(
            egui::RichText::new(
                "Nessun esame con una scheda: aprine uno da Zotero e salva la scheda \
                 per collegare il suo notebook.",
            )
            .size(12.0)
            .color(TENUE),
        );
        return;
    }

    // L'esame scelto resta fra un frame e l'altro; se punta a un esame che
    // non c'e' piu' si ricade sul primo.
    let scelto = stato
        .esame_notebooklm
        .clone()
        .filter(|e| esami.iter().any(|x| x == e))
        .unwrap_or_else(|| {
            // Meglio l'esame aperto da Zotero, se c'e'.
            let corrente = stato.nome_esame();
            if esami.iter().any(|e| e == &corrente) {
                corrente
            } else {
                esami[0].clone()
            }
        });
    stato.esame_notebooklm = Some(scelto.clone());

    ui.horizontal_wrapped(|ui| {
        egui::ComboBox::from_id_salt("esame_notebooklm")
            .width((crate::ui::larghezza(ui) * 0.5).clamp(120.0, 320.0))
            .selected_text(crate::ui::accorcia(&scelto, 28))
            .show_ui(ui, |ui| {
                for esame in &esami {
                    let etichetta = crate::ui::accorcia(esame, 34);
                    ui.selectable_value(
                        &mut stato.esame_notebooklm,
                        Some(esame.clone()),
                        etichetta,
                    );
                }
            });
        let collegato = stato
            .config
            .notebook_esami
            .get(&scelto)
            .map(|l| !l.trim().is_empty())
            .unwrap_or(false);
        if collegato {
            badge(ui, "notebook collegato", VERDE);
        } else {
            badge(ui, "senza link", TENUE);
        }
    });

    ui.add_space(4.0);
    ui.label(
        egui::RichText::new(
            "Scrivi la domanda: l'app aggiunge il contesto dell'esame e apre il \
             notebook giusto. Poi incolla qui la risposta per tenerla.",
        )
        .size(12.0)
        .color(TENUE),
    );

    egui::TextEdit::singleline(&mut stato.campo_domanda)
        .hint_text("Fammi un riassunto del capitolo 3")
        .desired_width(f32::INFINITY)
        .show(ui);
    let ha_domanda = !stato.campo_domanda.trim().is_empty();

    ui.horizontal_wrapped(|ui| {
        if ui
            .add_enabled(
                ha_domanda,
                egui::Button::new("❓ Chiedi a NotebookLM").min_size(egui::vec2(0.0, 30.0)),
            )
            .clicked()
        {
            chiedi(stato, &scelto);
        }
        if ui.button("📋 Incolla la risposta").clicked() {
            incolla_risposta(stato, &scelto);
        }
    });

    if let Some((esame_risposta, testo)) = stato.risposta_notebooklm.clone() {
        egui::Frame::none()
            .fill(crate::ui::SFONDO)
            .stroke(egui::Stroke::new(1.0, ACCENTO.gamma_multiply(0.5)))
            .rounding(egui::Rounding::same(8.0))
            .inner_margin(egui::Margin::same(10.0))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new("💡 Risposta di NotebookLM")
                            .size(13.0)
                            .strong()
                            .color(ACCENTO),
                    );
                    badge(ui, &esame_risposta, VERDE);
                    if ui.small_button("✕").clicked() {
                        stato.risposta_notebooklm = None;
                    }
                });
                egui::ScrollArea::vertical()
                    .max_height(220.0)
                    .show(ui, |ui| {
                        ui.label(egui::RichText::new(&testo).size(14.0));
                    });
                if ui.button("💾 Salva come carta di ripasso").clicked() {
                    salva_carta(stato, &esame_risposta, &testo);
                }
            });
    }
}

/// Compone la domanda, la copia e apre il notebook dell'esame.
fn chiedi(stato: &mut StatoApp, esame: &str) {
    let domanda = stato.campo_domanda.trim().to_string();
    let messaggio = crate::notebooklm::prepara_domanda(esame, &domanda);
    if messaggio.is_empty() {
        return;
    }
    if let Err(e) = crate::notebooklm::copia_negli_appunti(&messaggio) {
        stato.avviso = Some(e);
        return;
    }
    // La domanda resta in memoria: sara' la faccia della carta di ripasso
    // quando incolli la spiegazione.
    stato.domanda_notebooklm = domanda;
    match stato.config.notebook_esami.get(esame) {
        Some(link) if !link.trim().is_empty() => {
            // Se il notebook e' gia' aperto nel browser si porta avanti
            // quello: aprire una scheda in piu' sullo stesso notebook
            // confonde e fa perdere la conversazione.
            if !crate::finestre::porta_avanti_notebooklm() {
                if let Err(e) = crate::notebooklm::apri(link) {
                    stato.avviso = Some(e);
                    return;
                }
            }
            stato.aggiungi_notifica("Domanda copiata: incolla e premi Invio su NotebookLM.");
        }
        _ => {
            // Senza link salvato si apre la pagina iniziale: l'utente sceglie
            // il notebook, e il link lo salva per la prossima volta.
            let _ = crate::notebooklm::apri("https://notebooklm.google.com/");
            stato.aggiungi_notifica(
                "Domanda copiata: incolla e premi Invio. Salva il link del notebook per la prossima volta.",
            );
        }
    }
}

/// Riporta dentro l'app la risposta copiata da NotebookLM.
fn incolla_risposta(stato: &mut StatoApp, esame: &str) {
    match crate::audio::MotoreAudio::leggi_clipboard() {
        Ok(testo) if !testo.trim().is_empty() => {
            stato.risposta_notebooklm = Some((esame.to_string(), testo));
            stato.avviso = None;
        }
        Ok(_) => stato.avviso = Some(
            "Il clipboard e' vuoto. Copia la risposta da NotebookLM e riprova.".into(),
        ),
        Err(e) => stato.avviso = Some(format!("Non riesco a leggere il clipboard: {e}")),
    }
}

/// Salva la risposta di NotebookLM come carta di ripasso dell'esame.
///
/// La carta e' fatta al contrario di come verrebbe naturale: davanti ci va
/// la domanda che hai fatto, dietro la spiegazione che hai ottenuto. Cosi'
/// fra qualche giorno l'app ti richiede quella domanda e sei tu a dover
/// ricostruire la risposta: e' il momento in cui la spiegazione entra
/// davvero nella memoria.
fn salva_carta(stato: &mut StatoApp, esame: &str, testo: &str) {
    let domanda = componi_domanda_carta(&stato.domanda_notebooklm, esame, testo);
    match crate::carte::aggiungi(esame, &domanda, testo) {
        Ok(()) => {
            stato.aggiungi_notifica("Carta salvata: la risposta torna nel ripasso.");
            stato.riepilogo_notebooklm();
        }
        Err(e) => stato.avviso = Some(format!("Non riesco a salvare la carta: {e}")),
    }
}

/// La faccia della carta di ripasso.
///
/// Con una domanda registrata si usa quella (e' la domanda che hai fatto
/// davvero); senza, si ripiega sull'inizio della spiegazione: meglio una
/// carta imperfetta che una spiegazione persa.
///
/// La domanda viene tagliata a un confine di parola: incollare tre paragrafi
/// come domanda produrrebbe una carta illeggibile.
fn componi_domanda_carta(domanda_notebooklm: &str, esame: &str, testo: &str) -> String {
    if domanda_notebooklm.trim().is_empty() {
        format!(
            "Spiegami questo passaggio di {esame}: {}",
            crate::ui::accorcia(testo, 200)
        )
    } else {
        format!(
            "{} (esame di {esame})",
            crate::ui::accorcia(domanda_notebooklm.trim(), MASSIMO_DOMANDA)
        )
    }
}

#[cfg(test)]
mod test {
    use super::componi_domanda_carta;

    /// La domanda che hai scritto e' la faccia della carta: senza, il
    /// ripasso mostrerebbe un pezzo della risposta e la carta non
    /// interrogherebbe niente.
    #[test]
    fn la_carta_usa_la_domanda_che_hai_fatto() {
        let domanda = componi_domanda_carta(
            "Fammi un riassunto del capitolo 3",
            "Storia romana",
            "Il capitolo parla della riforma agraria.",
        );

        assert!(domanda.contains("Fammi un riassunto del capitolo 3"));
        assert!(domanda.contains("Storia romana"));
    }

    /// Senza una domanda registrata la carta resta utile: usa l'inizio
    /// della spiegazione invece di perdere il materiale.
    #[test]
    fn senza_domanda_la_carta_usa_la_spiegazione() {
        let domanda = componi_domanda_carta("", "Storia romana", "La riforma agraria.");

        assert!(domanda.contains("Spiegami questo passaggio"));
        assert!(domanda.contains("Storia romana"));
        assert!(domanda.contains("La riforma agraria"));
    }

    /// Una domanda lunghissima non deve produrre una carta illeggibile.
    ///
    /// La domanda viene tagliata a un confine di parola: se restasse intera
    /// il ripasso mostrerebbe un muro di testo.
    #[test]
    fn una_domanda_lunga_viene_tagliata() {
        let lunga = "parola ".repeat(200);
        let domanda = componi_domanda_carta(&lunga, "Storia", "risposta");

        assert!(
            domanda.chars().count() < lunga.chars().count(),
            "la domanda non e' stata tagliata: {} caratteri",
            domanda.chars().count()
        );
        assert!(domanda.contains("Storia"));
        // Il taglio cade a confine di parola, non a meta' di una.
        assert!(!domanda.ends_with("parol"));
    }
}
