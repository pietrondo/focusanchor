//! Chiedere a NotebookLM e ricevere la risposta dentro l'app.
//!
//! Si appoggia alla CLI di `notebooklm-py`: la domanda va al notebook
//! dell'esame, la risposta arriva in JSON e viene mostrata qui. Da li' puo'
//! diventare una carta di ripasso, cosi' la spiegazione non si perde.
//!
//! Non serve Zotero: gli esami sono quelli con una scheda in `config.esami`.

use eframe::egui;

use crate::state::StatoApp;
use crate::ui::{badge, ACCENTO, GIALLO, TENUE, VERDE};

/// Lunghezza massima della domanda che diventa la faccia di una carta.
const MASSIMO_DOMANDA: usize = 240;

/// Pannello "Chiedi a NotebookLM": domanda, invio e risposta.
///
/// L'esame arriva dall'esterno: il selettore e' uno solo per tutta la
/// finestra, altrimenti Zotero e NotebookLM possono parlare di due esami
/// diversi nello stesso momento.
pub fn sezione_chiedi(ui: &mut egui::Ui, stato: &mut StatoApp, esame: &str) {
    if esame.trim().is_empty() {
        return;
    }
    let scelto = esame.to_string();

    ui.horizontal_wrapped(|ui| {
        if notebook_di(stato, &scelto).is_some() {
            badge(ui, "notebook collegato", VERDE);
        } else {
            badge(ui, "senza notebook", GIALLO);
        }
    });

    // Collegare i cinque esami a mano sarebbe cinque copia-incolla: la CLI
    // sa i titoli dei notebook, quindi si abbinano per nome.
    ui.horizontal_wrapped(|ui| {
        if ui
            .button("🔗 Collega i notebook per nome")
            .on_hover_text("Legge i tuoi notebook da NotebookLM e li abbina agli esami")
            .clicked()
        {
            collega_notebook(stato);
        }
        if notebook_di(stato, &scelto).is_some() && ui.button("▶ Apri nel browser").clicked() {
            if let Some(link) = stato.config.notebook_esami.get(&scelto).cloned() {
                if let Err(e) = crate::notebooklm::apri(&link) {
                    stato.avviso = Some(e);
                }
            }
        }
    });

    ui.add_space(4.0);
    let domanda = egui::TextEdit::singleline(&mut stato.campo_domanda)
        .hint_text("Fammi un riassunto del capitolo 3")
        .desired_width(f32::INFINITY)
        .show(ui);
    // Invio manda la domanda: e' il gesto naturale dopo averla scritta.
    let invio = domanda.response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
    let pronta = !stato.campo_domanda.trim().is_empty();

    ui.horizontal_wrapped(|ui| {
        let collegato = notebook_di(stato, &scelto).is_some();
        let attivo = pronta && collegato && !stato.chiedi.in_corso;
        if ui
            .add_enabled(
                attivo,
                egui::Button::new("❓ Chiedi a NotebookLM").min_size(egui::vec2(0.0, 30.0)),
            )
            .on_disabled_hover_text(if collegato {
                "Scrivi prima la domanda"
            } else {
                "Collega il notebook di questo esame"
            })
            .clicked()
            || (invio && attivo)
        {
            chiedi(stato, &scelto);
        }
        // Ripiego a mano: se la CLI si rompe (la libreria non e' ufficiale)
        // il flusso col copia-incolla funziona ancora.
        if ui
            .small_button("copia la domanda")
            .on_hover_text("Ripiego a mano: copia la domanda e incollala in NotebookLM")
            .clicked()
        {
            copia_domanda(stato, &scelto);
        }
    });

    if stato.chiedi.in_corso {
        ui.horizontal(|ui| {
            ui.spinner();
            ui.label(
                egui::RichText::new("NotebookLM sta leggendo i testi…")
                    .size(13.0)
                    .color(TENUE),
            );
        });
        return;
    }

    mostra_risposta(ui, stato);
    ui.add_space(6.0);
    strumenti(ui, stato, &scelto);
}

/// Fonti del notebook e generazione delle flashcard.
///
/// Sono le due cose che prima costringevano ad aprire il browser: caricare
/// il materiale e portarsi via le carte generate.
fn strumenti(ui: &mut egui::Ui, stato: &mut StatoApp, esame: &str) {
    let notebook = notebook_di(stato, esame);

    ui.separator();
    ui.horizontal_wrapped(|ui| {
        let collegato = notebook.is_some();
        if ui
            .add_enabled(collegato, egui::Button::new("📄 Fonti"))
            .on_hover_text("Vedi cosa hai gia' caricato in questo notebook")
            .clicked()
        {
            if let Some(id) = &notebook {
                stato.genera.fonti(id, esame);
            }
        }
        if ui
            .add_enabled(collegato, egui::Button::new("⬆ Carica la scheda"))
            .on_hover_text(
                "Genera la scheda dell'esame e la carica come fonte, senza trascinarla nel browser",
            )
            .clicked()
        {
            carica_scheda(stato, esame);
        }
        if ui
            .add_enabled(collegato, egui::Button::new("🎴 Genera flashcard"))
            .on_hover_text("NotebookLM scrive un mazzo dalle fonti; poi lo importi nel ripasso")
            .clicked()
        {
            if let Some(id) = &notebook {
                stato.genera.flashcard(id, esame);
            }
        }
    });

    if stato.genera.in_corso {
        ui.horizontal(|ui| {
            ui.spinner();
            ui.label(
                egui::RichText::new("NotebookLM sta lavorando…")
                    .size(13.0)
                    .color(TENUE),
            );
        });
        return;
    }

    match stato.genera.esito.clone() {
        Some(crate::notebooklm::Generazione::Fonti(fonti)) => {
            if fonti.is_empty() {
                ui.label(
                    egui::RichText::new("Il notebook non ha ancora fonti: carica la scheda.")
                        .size(12.0)
                        .color(TENUE),
                );
            }
            for fonte in fonti.iter().take(8) {
                ui.horizontal_wrapped(|ui| {
                    ui.label(
                        egui::RichText::new(crate::ui::accorcia(&fonte.titolo, 40)).size(13.0),
                    );
                    if !fonte.tipo.is_empty() {
                        badge(ui, &fonte.tipo, TENUE);
                    }
                    if fonte.pronta {
                        badge(ui, "pronta", VERDE);
                    } else {
                        badge(ui, "in elaborazione", GIALLO);
                    }
                });
            }
            ui.label(
                egui::RichText::new(format!("{} fonti in tutto.", fonti.len()))
                    .size(12.0)
                    .color(TENUE),
            );
        }
        Some(crate::notebooklm::Generazione::Flashcard(carte)) => {
            ui.label(
                egui::RichText::new(format!(
                    "{} carte generate dalle fonti del notebook.",
                    carte.len()
                ))
                .size(13.0)
                .color(TENUE),
            );
            if ui
                .add(
                    egui::Button::new(format!("📥 Importa {} carte nel ripasso", carte.len()))
                        .min_size(egui::vec2(0.0, 30.0)),
                )
                .clicked()
            {
                importa_carte(stato, esame, &carte);
                stato.genera.pulisci();
            }
        }
        Some(crate::notebooklm::Generazione::Fatto(messaggio)) => {
            ui.label(egui::RichText::new(messaggio).size(12.0).color(VERDE));
        }
        Some(crate::notebooklm::Generazione::Errore(messaggio)) => {
            ui.label(egui::RichText::new(messaggio).size(12.0).color(GIALLO));
        }
        None => {}
    }
}

/// Importa nel ripasso le carte generate da NotebookLM.
fn importa_carte(stato: &mut StatoApp, esame: &str, carte: &[crate::notebooklm::Flashcard]) {
    let mut importate = 0;
    for carta in carte {
        if crate::carte::aggiungi(esame, &carta.domanda, &carta.risposta).is_ok() {
            importate += 1;
        }
    }
    stato.riepilogo_notebooklm();
    stato.aggiungi_notifica(&format!("{importate} carte entrate nel ripasso."));
}

/// Genera la scheda dell'esame e la carica come fonte del notebook.
fn carica_scheda(stato: &mut StatoApp, esame: &str) {
    let percorso = crate::archivi::percorso_notebooklm_esame(esame);
    // Se la scheda non c'e' ancora, la si genera adesso: caricare un file
    // che non esiste e' un errore che l'utente non capirebbe.
    if !percorso.exists() {
        crate::ui_archivi::genera_notebooklm(stato);
    }
    let Some(notebook) = notebook_di(stato, esame) else {
        stato.avviso = Some(
            "Questo esame non ha un notebook collegato: usa «Collega i notebook per nome»."
                .into(),
        );
        return;
    };
    stato.genera.carica(&notebook, esame, percorso);
}

/// Identificativo del notebook dell'esame, ricavato dal link salvato.
fn notebook_di(stato: &StatoApp, esame: &str) -> Option<String> {
    stato
        .config
        .notebook_esami
        .get(esame)
        .and_then(|link| crate::notebooklm::id_da_link(link))
}

/// Abbina gli esami ai notebook leggendo l'elenco dall'account.
fn collega_notebook(stato: &mut StatoApp) {
    let notebook = match crate::notebooklm::elenco_notebook() {
        Ok(elenco) => elenco,
        Err(e) => {
            stato.avviso = Some(e);
            return;
        }
    };
    let mut collegati = 0;
    let esami: Vec<String> = stato.config.esami.keys().cloned().collect();
    for esame in esami {
        if let Some(id) = crate::notebooklm::abbina(&esame, &notebook) {
            stato
                .config
                .notebook_esami
                .insert(esame, crate::notebooklm::link_da_id(&id));
            collegati += 1;
        }
    }
    stato.salva_configurazione();
    if collegati == 0 {
        stato.avviso = Some(
            "Nessun notebook ha un titolo che somiglia a un esame. Rinomina il notebook \
             su NotebookLM o salva il link a mano."
                .into(),
        );
    } else {
        stato.aggiungi_notifica(&format!("{collegati} notebook collegati agli esami."));
    }
}

/// Manda la domanda al notebook dell'esame.
fn chiedi(stato: &mut StatoApp, esame: &str) {
    let domanda = stato.campo_domanda.trim().to_string();
    if domanda.is_empty() {
        return;
    }
    let Some(notebook) = notebook_di(stato, esame) else {
        stato.avviso = Some(
            "Questo esame non ha un notebook collegato: usa «Collega i notebook per nome»."
                .into(),
        );
        return;
    };
    // La domanda resta in memoria: sara' la faccia della carta di ripasso.
    stato.domanda_notebooklm = domanda.clone();
    stato.chiedi.chiedi(&notebook, esame, &domanda);
}

/// Ripiego a mano: copia la domanda pronta per essere incollata.
fn copia_domanda(stato: &mut StatoApp, esame: &str) {
    let messaggio = crate::notebooklm::prepara_domanda(esame, &stato.campo_domanda);
    if messaggio.is_empty() {
        return;
    }
    match crate::notebooklm::copia_negli_appunti(&messaggio) {
        Ok(()) => {
            stato.domanda_notebooklm = stato.campo_domanda.trim().to_string();
            stato.aggiungi_notifica("Domanda copiata: incollala in NotebookLM.");
        }
        Err(e) => stato.avviso = Some(e),
    }
}

/// Mostra la risposta arrivata, con il pulsante per salvarla come carta.
fn mostra_risposta(ui: &mut egui::Ui, stato: &mut StatoApp) {
    let Some(testo) = stato.chiedi.risposta.clone() else {
        return;
    };
    let esame = stato.chiedi.esame.clone();

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
                if !esame.is_empty() {
                    badge(ui, &crate::ui::accorcia(&esame, 24), VERDE);
                }
                if ui.small_button("✕").clicked() {
                    stato.chiedi.pulisci();
                }
            });
            egui::ScrollArea::vertical()
                .max_height(260.0)
                .show(ui, |ui| {
                    ui.label(egui::RichText::new(&testo).size(14.0));
                });
            ui.horizontal_wrapped(|ui| {
                if ui.button("💾 Salva come carta di ripasso").clicked() {
                    salva_carta(stato, &esame, &testo);
                }
                if ui.button("📋 Copia").clicked() {
                    match crate::notebooklm::copia_negli_appunti(&testo) {
                        Ok(()) => stato.aggiungi_notifica("Risposta copiata."),
                        Err(e) => stato.avviso = Some(e),
                    }
                }
            });
        });
}

/// Salva la risposta di NotebookLM come carta di ripasso dell'esame.
///
/// La carta e' fatta al contrario di come verrebbe naturale: davanti ci va
/// la domanda che hai fatto, dietro la spiegazione che hai ottenuto. Cosi'
/// fra qualche giorno l'app ti richiede quella domanda e sei tu a dover
/// ricostruire la risposta.
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
    /// ripasso mostrerebbe un pezzo della risposta e non interrogherebbe.
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
        assert!(!domanda.ends_with("parol"));
    }
}
