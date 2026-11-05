//! Spazio di lavoro dell'esame: materiali Zotero e notebook NotebookLM
//! nella stessa scheda, con un solo selettore d'esame.
//!
//! Prima erano due riquadri con due tendine diverse: si poteva avere
//! "Storia romana" a sinistra e "Letteratura italiana" a destra, e nessuno
//! se ne accorgeva. Qui l'esame e' uno solo e tutto il resto ne consegue.

use eframe::egui;

use crate::state::{StatoApp, MAX_TESTI};
use crate::ui::{badge, ARANCIO, TENUE, VERDE, VIOLA};

/// Elenco degli esami su cui lavorare.
///
/// Prima le schede salvate (gli esami veri); se non ce n'e' ancora nessuna
/// si usano le collezioni Zotero, cosi' il primo avvio non e' un vicolo
/// cieco.
fn esami_disponibili(stato: &StatoApp) -> Vec<String> {
    if !stato.config.esami.is_empty() {
        return stato.config.esami.keys().cloned().collect();
    }
    stato.collezioni.iter().map(|c| c.nome.clone()).collect()
}

/// Esame corrente: quello scelto, altrimenti quello aperto da Zotero.
fn esame_corrente(stato: &StatoApp, disponibili: &[String]) -> String {
    if let Some(scelto) = stato
        .esame_notebooklm
        .clone()
        .filter(|e| disponibili.iter().any(|x| x == e))
    {
        return scelto;
    }
    let da_zotero = stato.nome_esame();
    if disponibili.iter().any(|e| e == &da_zotero) {
        return da_zotero;
    }
    disponibili.first().cloned().unwrap_or_default()
}

/// Cambia esame in un colpo solo: nome, collezione Zotero e materiali.
fn scegli_esame(stato: &mut StatoApp, nome: &str) {
    stato.esame_notebooklm = Some(nome.to_string());
    // L'esame scelto si ricorda: alla prossima apertura i materiali sono
    // gia' caricati.
    if stato.config.esame_corrente != nome {
        stato.config.esame_corrente = nome.to_string();
        stato.salva_configurazione();
    }
    let id = stato
        .collezioni
        .iter()
        .find(|c| c.nome.eq_ignore_ascii_case(nome))
        .map(|c| c.id);
    if let Some(id) = id {
        // Senza snapshot Zotero non c'e' niente da caricare: si evita di
        // svuotare i materiali per un cambio che non porta dati.
        if stato.esame_scelto != Some(id) && stato.snapshot_zotero.is_some() {
            crate::ui_esami::cambia_esame(stato, Some(id));
        } else {
            stato.esame_scelto = Some(id);
        }
    }
}

/// Il selettore dell'esame, una riga.
pub fn selettore_esame(ui: &mut egui::Ui, stato: &mut StatoApp) {
    let disponibili = esami_disponibili(stato);
    if disponibili.is_empty() {
        return;
    }
    let corrente = esame_corrente(stato, &disponibili);
    stato.esame_notebooklm = Some(corrente.clone());
    let mut scelto = Some(corrente.clone());
    egui::ComboBox::from_id_salt("scegli_esame")
        // Larghezza protetta: se fosse infinita il menu romperebbe il layout.
        .width(crate::ui::larghezza(ui) - 8.0)
        .selected_text(egui::RichText::new(crate::ui::accorcia(&corrente, 40)).size(15.0))
        .show_ui(ui, |ui| {
            for nome in &disponibili {
                let collezione = stato
                    .collezioni
                    .iter()
                    .find(|c| c.nome.eq_ignore_ascii_case(nome));
                let etichetta = match collezione {
                    Some(c) if c.documenti > 0 => {
                        format!("{}  ·  {} doc.", nome, c.documenti)
                    }
                    _ => nome.clone(),
                };
                ui.selectable_value(&mut scelto, Some(nome.clone()), etichetta);
            }
        });
    if let Some(nuovo) = scelto {
        if nuovo != corrente {
            scegli_esame(stato, &nuovo);
        }
    }
}

/// Il tuo esame: tutto quello che serve su un esame, in una scheda sola.
pub fn sezione_esame(ui: &mut egui::Ui, stato: &mut StatoApp) {
    if esami_disponibili(stato).is_empty() {
        crate::ui_caricamento::stato_vuoto(ui, stato);
        return;
    }
    selettore_esame(ui, stato);
    let corrente = esame_corrente(stato, &esami_disponibili(stato));

    ui.add_space(8.0);
    materiali_zotero(ui, stato, &corrente);

    ui.add_space(8.0);
    crate::ui_scheda::sezione_scheda_esame(ui, stato, &corrente);

    ui.add_space(8.0);
    testi_da_alternare(ui, stato);

    ui.add_space(8.0);
    ripasso(ui, stato);
}

/// Nome dell'esame su cui sta lavorando la finestra.
///
/// Lo legge anche la colonna di destra: le due colonne devono parlare
/// dello stesso esame, altrimenti si torna al problema di prima.
pub fn nome_esame_corrente(stato: &StatoApp) -> String {
    esame_corrente(stato, &esami_disponibili(stato))
}

/// Riquadro dove vive la finestra di Zotero, se agganciata.
///
/// Lo spazio si prenota sempre: se non c'e' Zotero dentro ci sta il
/// pulsante per agganciarlo, e il layout non salta quando compare.
pub fn riquadro_zotero(ui: &mut egui::Ui, stato: &mut StatoApp, altezza: f32) {
    let larghezza = crate::ui::larghezza(ui);
    let (rettangolo, _) =
        ui.allocate_exact_size(egui::vec2(larghezza, altezza), egui::Sense::hover());
    // La posizione serve al ciclo principale per muovere la finestra vera.
    stato.area_zotero = Some(rettangolo);

    if stato.zotero_incassato.is_some() {
        // Lo spazio e' occupato dalla finestra vera: sotto resta solo il
        // comando per liberarla, che deve stare fuori dal riquadro.
        ui.horizontal_wrapped(|ui| {
            ui.label(
                egui::RichText::new("Zotero è qui dentro.")
                    .size(11.0)
                    .color(TENUE),
            );
            if ui.small_button("⏏ Stacca Zotero").clicked() {
                stacca_zotero(stato);
            }
        });
        return;
    }

    // Non agganciato: il riquadro ospita il pulsante.
    let mut dentro = ui.new_child(egui::UiBuilder::new()
        .max_rect(rettangolo)
        .layout(egui::Layout::top_down(egui::Align::Center)));
    dentro.add_space(altezza / 2.0 - 40.0);
    dentro.label(
        egui::RichText::new("📚 Zotero")
            .size(16.0)
            .strong()
            .color(TENUE),
    );
    dentro.label(
        egui::RichText::new("Apri i PDF e sottolinea, senza cambiare finestra.")
            .size(12.0)
            .color(TENUE),
    );
    dentro.add_space(8.0);
    if dentro
        .add(egui::Button::new("📌 Aggancia Zotero qui").min_size(egui::vec2(220.0, 32.0)))
        .clicked()
    {
        aggancia_zotero(stato);
    }
}

/// Aggancia la finestra di Zotero dentro la nostra.
fn aggancia_zotero(stato: &mut StatoApp) {
    // Zotero puo' essere chiuso: si prova ad aprirlo e si aspetta che la
    // finestra esista davvero, invece di fallire subito.
    let mut finestra = crate::incasso::finestra_zotero();
    if finestra.is_none() {
        if let Err(e) = crate::incasso::avvia_zotero() {
            stato.avviso = Some(e);
            return;
        }
        for _ in 0..30 {
            std::thread::sleep(std::time::Duration::from_millis(500));
            finestra = crate::incasso::finestra_zotero();
            if finestra.is_some() {
                break;
            }
        }
    }
    let Some(finestra) = finestra else {
        stato.avviso = Some(
            "Non trovo la finestra di Zotero. Aprilo e riprova.".into(),
        );
        return;
    };
    let Some(ospite) = crate::incasso::finestra_app() else {
        stato.avviso = Some("Non trovo la finestra di FocusAnchor.".into());
        return;
    };
    let area = stato.area_zotero.unwrap_or(egui::Rect::from_min_size(
        egui::Pos2::ZERO,
        egui::vec2(600.0, 360.0),
    ));
    if crate::incasso::aggancia(
        finestra,
        ospite,
        area.min.x as i32,
        area.min.y as i32,
        area.width() as i32,
        area.height() as i32,
    ) {
        stato.zotero_incassato = Some(finestra.0 as isize);
        stato.aggiungi_notifica("Zotero agganciato dentro la finestra.");
    } else {
        stato.avviso = Some("Windows non ha permesso di agganciare Zotero.".into());
    }
}

/// Stacca Zotero e lo rimette come finestra indipendente.
pub fn stacca_zotero(stato: &mut StatoApp) {
    if let Some(handle) = stato.zotero_incassato.take() {
        let finestra = windows::Win32::Foundation::HWND(handle as *mut core::ffi::c_void);
        crate::incasso::stacca(finestra);
        stato.aggiungi_notifica("Zotero staccato: è di nuovo una finestra sua.");
    }
}

/// Materiali Zotero: stato, ricarica e (se caricati) i documenti.
fn materiali_zotero(ui: &mut egui::Ui, stato: &mut StatoApp, esame: &str) {
    egui::CollapsingHeader::new("📚 Materiali Zotero")
        .default_open(true)
        .show(ui, |ui| {
            riquadro_zotero(ui, stato, 360.0);
            ui.add_space(4.0);
            crate::ui_caricamento::pulsanti_caricamento(ui, stato);
            ui.add_space(4.0);

            let collegato = stato.snapshot_zotero.is_some();
            let stesso_esame = stato
                .esame_scelto
                .and_then(|id| stato.collezioni.iter().find(|c| c.id == id))
                .map(|c| c.nome.eq_ignore_ascii_case(esame))
                .unwrap_or(false);

            ui.horizontal_wrapped(|ui| {
                let colore = if collegato { VERDE } else { TENUE };
                let testo = if !collegato {
                    "Zotero non caricato".to_string()
                } else if !stesso_esame {
                    "Questo esame non ha una collezione Zotero con lo stesso nome.".to_string()
                } else {
                    format!(
                        "{} documenti · {} annotazioni",
                        stato.documenti_esame.len(),
                        stato.evidenziazioni.len()
                    )
                };
                ui.label(egui::RichText::new(testo).size(12.0).color(colore));
                if collegato && ui.small_button("🔄 Ricarica").clicked() {
                    crate::ui_esami::ricarica_testi(stato);
                }
            });

            if stato.snapshot_zotero.is_some()
                && stato.esame_scelto.is_some()
                && stato.documenti_esame.is_empty()
            {
                ui.label(
                    egui::RichText::new("Nessun documento in questa collezione.")
                        .size(12.0)
                        .color(TENUE),
                );
            }
        });
}

/// Testi dell'esame, da spuntare in 2 o 3.
fn testi_da_alternare(ui: &mut egui::Ui, stato: &mut StatoApp) {
    if stato.documenti_esame.is_empty() {
        return;
    }
    egui::CollapsingHeader::new("📄 Testi da alternare")
        .default_open(true)
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new("Spunta 2 o 3 testi: la sessione ruoterà fra loro.")
                        .size(12.0)
                        .color(TENUE),
                );
                badge(
                    ui,
                    &format!("{}/{}", stato.testi_scelti.len(), MAX_TESTI),
                    VERDE,
                );
            });
            elenco_testi(ui, stato);
            if stato.testi_scelti.len() == 1 {
                ui.label(
                    egui::RichText::new("Con un solo testo non c'e' interleaving.")
                        .size(12.0)
                        .color(crate::ui::GIALLO),
                );
            }
        });
}

/// Le carte da ripassare per questo esame.
///
/// Il ripasso completo sta in "Altre opzioni"; qui serve il numero, perche'
/// "quante carte ho su questo esame" e' parte del quadro dell'esame.
fn ripasso(ui: &mut egui::Ui, stato: &mut StatoApp) {
    stato.sincronizza_memoria();
    let oggi = crate::carte::oggi();
    let scadute = stato.memoria.scadute(&oggi).len();
    let totale = stato.memoria.carte.len();

    ui.horizontal_wrapped(|ui| {
        ui.label(egui::RichText::new("🔁 Ripasso").size(13.0).color(TENUE));
        if totale == 0 {
            badge(ui, "nessuna carta", TENUE);
        } else {
            badge(ui, &format!("{totale} carte"), VIOLA);
            let colore = if scadute == 0 { VERDE } else { ARANCIO };
            badge(ui, &format!("{scadute} da ripassare"), colore);
        }
    });
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