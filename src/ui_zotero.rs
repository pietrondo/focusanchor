//! Selezione dell'esame e dei testi da alternare, presi da Zotero.

use eframe::egui;

use crate::state::{StatoApp, MAX_TESTI};
use crate::ui::{badge, TENUE, VERDE};

/// Sezione "Esame e testi": due menu in sequenza.
pub fn sezione_esame(ui: &mut egui::Ui, stato: &mut StatoApp) {
    if stato.collezioni.is_empty() {
        stato_vuoto(ui, stato);
        return;
    }

    pulsanti_caricamento(ui, stato);
    ui.add_space(10.0);

    // Primo menu: l'esame, cioe' la collezione Zotero.
    ui.label(
        egui::RichText::new("ESAME")
            .size(11.0)
            .strong()
            .color(TENUE),
    );
    let esame_corrente = stato
        .esame_scelto
        .and_then(|id| stato.collezioni.iter().find(|c| c.id == id))
        .map(|c| c.nome.clone())
        .unwrap_or_else(|| "Scegli l'esame…".to_string());

    let mut nuovo_esame = stato.esame_scelto;
    egui::ComboBox::from_id_salt("scegli_esame")
        // Larghezza protetta: se fosse infinita il menu romperebbe il layout.
        .width(crate::ui::larghezza(ui) - 8.0)
        .selected_text(egui::RichText::new(esame_corrente).size(15.0))
        .show_ui(ui, |ui| {
            for collezione in &stato.collezioni {
                let etichetta = if collezione.sottocollezioni > 0 {
                    format!(
                        "{}  ·  {} doc. ({} sottocollezioni)",
                        collezione.nome, collezione.documenti, collezione.sottocollezioni
                    )
                } else {
                    format!("{}  ·  {} doc.", collezione.nome, collezione.documenti)
                };
                ui.selectable_value(&mut nuovo_esame, Some(collezione.id), etichetta);
            }
        });
    if nuovo_esame != stato.esame_scelto {
        crate::ui_esami::cambia_esame(stato, nuovo_esame);
    }

    ui.add_space(6.0);
    sezione_notebook(ui, stato);

    ui.add_space(12.0);
    sezione_testi(ui, stato);
}

/// Link del notebook NotebookLM dell'esame scelto.
///
/// Ogni esame ha il suo notebook con i testi caricati: salvando il link una
/// volta, "NotebookLM" apre direttamente quello invece della pagina iniziale.
fn sezione_notebook(ui: &mut egui::Ui, stato: &mut StatoApp) {
    let Some(esame) = stato
        .esame_scelto
        .and_then(|id| stato.collezioni.iter().find(|c| c.id == id))
        .map(|c| c.nome.clone())
    else {
        return;
    };

    // Il campo lavora su una copia: si salva solo se il link e' valido.
    let salvato = stato
        .config
        .notebook_esami
        .get(&esame)
        .cloned()
        .unwrap_or_default();
    // Il testo si modifica direttamente nello stato: un buffer locale
    // verrebbe ricreato a ogni frame e l'incolla sparirebbe.
    let mut salva = false;
    let mut apri = false;

    egui::CollapsingHeader::new("📓 Notebook NotebookLM di questo esame")
        .default_open(salvato.is_empty())
        .show(ui, |ui| {
            ui.label(
                egui::RichText::new(
                    "Incolla il link del notebook dove hai caricato i testi dell'esame. \
                     Verra' aperto quando premi «NotebookLM» in sessione.",
                )
                .size(12.0)
                .color(TENUE),
            );
            egui::TextEdit::singleline(&mut stato.campo_notebook)
                .hint_text("https://notebooklm.google.com/notebook/…")
                .desired_width(f32::INFINITY)
                .show(ui);
            ui.horizontal(|ui| {
                if ui.button("💾 Salva link").clicked() {
                    salva = true;
                }
                if !salvato.is_empty() && ui.button("▶ Apri").clicked() {
                    apri = true;
                }
            });
        });

    if salva {
        match crate::notebooklm::normalizza_link(&stato.campo_notebook) {
            Some(link) => {
                stato.campo_notebook = link.clone();
                stato.config.notebook_esami.insert(esame, link);
                stato.salva_configurazione();
                stato.aggiungi_notifica("Link del notebook salvato.");
            }
            None => {
                stato.avviso = Some(
                    "Il link non sembra valido: incolla un indirizzo che inizi con https://".into(),
                );
            }
        }
    }
    if apri {
        if let Err(e) = crate::notebooklm::apri(&salvato) {
            stato.avviso = Some(e);
        }
    }
}

/// Stato vuoto: nessun esame caricato, con le due azioni in evidenza.
fn stato_vuoto(ui: &mut egui::Ui, stato: &mut StatoApp) {
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
fn pulsanti_caricamento(ui: &mut egui::Ui, stato: &mut StatoApp) {
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

/// Secondo menu: i testi dell'esame, da spuntare in 2 o 3.
fn sezione_testi(ui: &mut egui::Ui, stato: &mut StatoApp) {
    if stato.esame_scelto.is_none() {
        return;
    }
    if stato.documenti_esame.is_empty() {
        ui.label(
            egui::RichText::new("Questo esame non ha documenti con un titolo.")
                .size(13.0)
                .color(TENUE),
        );
        return;
    }

    ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new("Testi da alternare")
                .size(14.0)
                .strong(),
        );
        badge(
            ui,
            &format!("{}/{}", stato.testi_scelti.len(), MAX_TESTI),
            VERDE,
        );
        if ui
            .small_button("🔄")
            .on_hover_text("Ricarica i testi: utile se ne hai aggiunti in Zotero")
            .clicked()
        {
            crate::ui_esami::ricarica_testi(stato);
        }
    });
    ui.label(
        egui::RichText::new(
            "Spunta 2 o 3 testi: la sessione ruoterà fra loro ogni 10-12 minuti.",
        )
        .size(12.0)
        .color(TENUE),
    );
    ui.add_space(4.0);

    elenco_testi(ui, stato);

    if stato.testi_scelti.len() == 1 {
        ui.label(
            egui::RichText::new("Con un solo testo non c'e' interleaving: aggiungine un altro.")
                .size(12.0)
                .color(crate::ui::GIALLO),
        );
    }
}

/// Lista dei testi con la casella e il pulsante "apri in Zotero".
fn elenco_testi(ui: &mut egui::Ui, stato: &mut StatoApp) {
    // Le modifiche si raccolgono prima e si applicano dopo: durante il ciclo
    // `stato.documenti_esame` e' preso in prestito.
    let mut da_alternare: Option<String> = None;
    let mut da_aprire: Option<String> = None;
    egui::ScrollArea::vertical().max_height(190.0).show(ui, |ui| {
        for documento in &stato.documenti_esame {
            let mut spuntato = stato.testo_scelto(&documento.titolo);
            let etichetta = accorcia(&documento.titolo, 70);
            let suggerimento = format!(
                "{}\n{} · {}",
                documento.titolo,
                documento.tipo,
                documento.data_aggiunta.get(0..10).unwrap_or("")
            );
            ui.horizontal(|ui| {
                if ui
                    .checkbox(&mut spuntato, etichetta)
                    .on_hover_text(&suggerimento)
                    .changed()
                {
                    da_alternare = Some(documento.titolo.clone());
                }
                if ui
                    .small_button("↗")
                    .on_hover_text("Apri questo documento in Zotero")
                    .clicked()
                {
                    da_aprire = Some(documento.chiave.clone());
                }
            });
        }
    });
    if let Some(titolo) = da_alternare {
        stato.alterna_testo(&titolo);
    }
    if let Some(chiave) = da_aprire {
        if let Err(e) = crate::zotero::apri_in_zotero(&chiave) {
            stato.avviso = Some(e);
        }
    }
}

/// Accorcia un titolo lungo senza spezzare le parole a meta'.
fn accorcia(testo: &str, massimo: usize) -> String {
    if testo.chars().count() <= massimo {
        return testo.to_string();
    }
    let tagliato: String = testo.chars().take(massimo).collect();
    match tagliato.rfind(' ') {
        Some(pos) => format!("{}…", &tagliato[..pos]),
        None => format!("{tagliato}…"),
    }
}
