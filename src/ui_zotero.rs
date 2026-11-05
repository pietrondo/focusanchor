//! Selezione dell'esame e dei testi da alternare, presi da Zotero.

use eframe::egui;

use crate::state::{StatoApp, MAX_TESTI};
use crate::ui::{badge, TENUE, VERDE};

/// Sezione "Esame e testi": due menu in sequenza.
pub fn sezione_esame(ui: &mut egui::Ui, stato: &mut StatoApp) {
    if stato.collezioni.is_empty() {
        crate::ui_caricamento::stato_vuoto(ui, stato);
        return;
    }

    ui.label(
        egui::RichText::new("MATERIALI")
            .size(11.0)
            .strong()
            .color(TENUE),
    );
    crate::ui_caricamento::pulsanti_caricamento(ui, stato);
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

    ui.horizontal_wrapped(|ui| {
        let collegato = stato.snapshot_zotero.is_some();
        let colore = if collegato { VERDE } else { TENUE };
        let stato_zotero = if collegato {
            format!(
                "Zotero · {} documenti · {} annotazioni",
                stato.documenti_esame.len(),
                stato.evidenziazioni.len()
            )
        } else {
            "Zotero non caricato".to_string()
        };
        ui.label(egui::RichText::new(stato_zotero).size(12.0).color(colore));
        if ui.small_button("🔄 Ricarica da Zotero").clicked() {
            crate::ui_esami::ricarica_testi(stato);
        }
    });

    ui.add_space(6.0);
    sezione_notebook(ui, stato);

    ui.add_space(6.0);
    crate::ui_scheda::sezione_scheda_esame(ui, stato);

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

    egui::CollapsingHeader::new("📓 NotebookLM e pacchetto di studio")
        .default_open(true)
        .show(ui, |ui| {
            ui.label(
                egui::RichText::new(
                    "Il link è salvato per questo esame. Da qui puoi aprire il notebook \
                     e preparare una scheda da caricare manualmente come fonte.",
                )
                .size(12.0)
                .color(TENUE),
            );
            egui::TextEdit::singleline(&mut stato.campo_notebook)
                .hint_text("https://notebooklm.google.com/notebook/…")
                .desired_width(f32::INFINITY)
                .show(ui);
            ui.horizontal_wrapped(|ui| {
                if ui.button("💾 Salva link").clicked() {
                    salva = true;
                }
                if !salvato.is_empty() && ui.button("▶ Apri NotebookLM").clicked() {
                    apri = true;
                }
                if ui.button("Crea / aggiorna scheda").clicked() {
                    crate::ui_archivi::genera_notebooklm(stato);
                }
                if ui.button("Anteprima scheda").clicked() {
                    match crate::archivi::leggi_notebooklm_esame_result(&esame) {
                        Ok(testo) if !testo.trim().is_empty() => {
                            stato.anteprima_notebooklm =
                                Some((crate::archivi::nome_file_notebooklm(&esame), testo));
                            stato.avviso = None;
                        }
                        Ok(_) => crate::ui_archivi::segnala_errore_materiali(
                            stato,
                            "La scheda è vuota. Generala di nuovo prima dell'anteprima.",
                        ),
                        Err(e) => crate::ui_archivi::segnala_errore_materiali(stato, &e),
                    }
                }
                if ui.button("Copia scheda").clicked() {
                    match crate::archivi::leggi_notebooklm_esame_result(&esame) {
                        Ok(testo) if !testo.trim().is_empty() => {
                            match crate::notebooklm::copia_negli_appunti(&testo) {
                                Ok(()) => crate::ui_archivi::segnala_successo_materiali(
                                    stato,
                                    "Scheda copiata negli appunti.",
                                ),
                                Err(e) => crate::ui_archivi::segnala_errore_materiali(stato, &e),
                            }
                        }
                        Ok(_) => crate::ui_archivi::segnala_errore_materiali(
                            stato,
                            "La scheda è vuota. Generala di nuovo prima di copiarla.",
                        ),
                        Err(e) => crate::ui_archivi::segnala_errore_materiali(stato, &e),
                    }
                }
                let percorso = crate::archivi::percorso_notebooklm_esame(&esame);
                if percorso.exists() && ui.button("Mostra file").clicked() {
                    match crate::archivi::rivela_file(&percorso) {
                        Ok(()) => stato.avviso = None,
                        Err(e) => stato.avviso = Some(e),
                    }
                }
            });
            let mut chiudi_anteprima = false;
            if let Some((nome, contenuto)) = stato.anteprima_notebooklm.as_ref() {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("Anteprima scheda").strong());
                    if ui.small_button("Chiudi anteprima").clicked() {
                        chiudi_anteprima = true;
                    }
                });
                crate::ui::mostra_contenuto(ui, nome, contenuto);
            }
            if chiudi_anteprima {
                stato.anteprima_notebooklm = None;
            }
            ui.label(
                egui::RichText::new(
                    "La scheda include le annotazioni Zotero disponibili e i riassunti. \
                     NotebookLM non viene caricato automaticamente: dopo averla generata, \
                     aggiungi il file come fonte nel notebook.",
                )
                .size(12.0)
                .color(TENUE),
            );
        });

    if salva {
        // Il link puo' arrivare come indirizzo pulito oppure sparso in una
        // frase ("guarda questo notebooklm.google.com/notebook/xyz"): in
        // entrambi i casi si riconosce.
        let riconosciuto = crate::notebooklm::riconosci_link(&stato.campo_notebook);
        match riconosciuto.or_else(|| crate::notebooklm::normalizza_link(&stato.campo_notebook)) {
            Some(link) => {
                stato.campo_notebook = link.clone();
                salva_link_con_persistenza(stato, &esame, &link, |stato| {
                    stato.salva_configurazione_con_esito()
                });
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
        } else {
            crate::ui_archivi::segnala_successo_materiali(stato, "NotebookLM aperto nel browser.");
        }
    }
}

fn salva_link_con_persistenza(
    stato: &mut StatoApp,
    esame: &str,
    link: &str,
    persisti: impl FnOnce(&mut StatoApp) -> Result<(), String>,
) {
    let precedente = stato
        .config
        .notebook_esami
        .insert(esame.to_string(), link.to_string());
    let esito = persisti(stato);
    if esito.is_err() {
        if let Some(link_precedente) = precedente {
            stato
                .config
                .notebook_esami
                .insert(esame.to_string(), link_precedente);
        } else {
            stato.config.notebook_esami.remove(esame);
        }
    }
    match esito {
        Ok(()) => crate::ui_archivi::segnala_successo_materiali(
            stato,
            "Link del notebook salvato.",
        ),
        Err(e) => {
            stato.conferma = None;
            stato.avviso = Some(format!("Link non salvato: {e}"));
        }
    }
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
    let mut da_aprire: Option<(String, bool)> = None;
    egui::ScrollArea::vertical().max_height(190.0).show(ui, |ui| {
        for documento in &stato.documenti_esame {
            let mut spuntato = stato.testo_scelto(&documento.titolo);
            let etichetta = crate::ui::accorcia(&documento.titolo, 70);
            let suggerimento = format!(
                "{}\n{} · {}",
                documento.titolo,
                documento.tipo,
                documento.data_aggiunta.get(0..10).unwrap_or("")
            );
            // Se c'e' il PDF il pulsante lo apre direttamente: selezionare il
            // libro nella libreria era un passaggio in piu' con un click in piu'.
            let (chiave, e_pdf) = match &documento.chiave_pdf {
                Some(chiave_pdf) => (chiave_pdf.clone(), true),
                None => (documento.chiave.clone(), false),
            };
            let (simbolo, aiuto) = if e_pdf {
                ("📄", "Apri il PDF in Zotero")
            } else {
                ("↗", "Seleziona questo testo in Zotero")
            };
            ui.horizontal(|ui| {
                if ui
                    .checkbox(&mut spuntato, etichetta)
                    .on_hover_text(&suggerimento)
                    .changed()
                {
                    da_alternare = Some(documento.titolo.clone());
                }
                if ui
                    .small_button(simbolo)
                    .on_hover_text(aiuto)
                    .clicked()
                {
                    da_aprire = Some((chiave, e_pdf));
                }
            });
        }
    });
    if let Some(titolo) = da_alternare {
        stato.alterna_testo(&titolo);
    }
    if let Some((chiave, e_pdf)) = da_aprire {
        let esito = if e_pdf {
            crate::zotero::apri_pdf_in_zotero(&chiave)
        } else {
            crate::zotero::apri_in_zotero(&chiave)
        };
        if let Err(e) = esito {
            stato.avviso = Some(e);
        }
    }
}

#[cfg(test)]
mod test_salvataggio_link {
    use super::*;

    #[test]
    fn un_errore_di_persistenza_non_mostra_successo() {
        let mut stato = StatoApp::nuovo();
        stato.conferma = Some(("Link del notebook salvato.".into(), std::time::Instant::now()));
        stato.config.notebook_esami.insert(
            "Storia romana".into(),
            "https://notebooklm.google.com/old".into(),
        );

        salva_link_con_persistenza(
            &mut stato,
            "Storia romana",
            "https://notebooklm.google.com/new",
            |_| Err("accesso negato".into()),
        );

        assert!(stato.conferma.is_none());
        assert_eq!(
            stato.config.notebook_esami.get("Storia romana").unwrap(),
            "https://notebooklm.google.com/old"
        );
        assert!(stato
            .avviso
            .as_deref()
            .is_some_and(|avviso| avviso.contains("accesso negato")));
    }
}
