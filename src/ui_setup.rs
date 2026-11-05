//! Modalita' Configurazione: esame, testi, durata e audio.

use std::sync::mpsc::Sender;

use eframe::egui;

use crate::audio::MotoreAudio;
use crate::blocker::Segnale;
use crate::state::{StatoApp, DURATA_MAXIMA};
use crate::ui::{avviso, barra_xp, notifica, scheda, ACCENTO, ARANCIO, GIALLO, TENUE, VERDE, VIOLA};

/// Disegna la modalita' Configurazione (prima della sessione).
pub fn disegna(
    ctx: &egui::Context,
    stato: &mut StatoApp,
    audio: &mut MotoreAudio,
    mittente: &Sender<Segnale>,
) {
    egui::CentralPanel::default()
        .frame(
            egui::Frame::none()
                .fill(crate::ui::SFONDO)
                .inner_margin(egui::Margin::symmetric(18.0, 12.0)),
        )
        .show(ctx, |ui| {
            ui.add_space(2.0);
            intestazione(ui, stato);
            ui.add_space(10.0);
            pulsante_avvia(ui, stato, audio, mittente);
            ui.add_space(6.0);

            // La larghezza si calcola qui fuori: dentro lo ScrollArea
            // potrebbe essere infinita.
            let l = crate::ui::larghezza_schede(ui);

            if let Some(r) = &stato.ultimo_riepilogo {
                scheda(ui, l, "✓", "Sessione completata", VERDE, |ui| {
                    ui.label(egui::RichText::new(&r.esame).strong());
                    ui.label(format!(
                        "{} min · {} XP · {} distrazioni · {} blocchi",
                        r.minuti, r.xp, r.distrazioni, r.blocchi
                    ));
                    if !r.tempo_documenti.is_empty() {
                        ui.label("Tempo per documento:");
                        for (titolo, secondi) in &r.tempo_documenti {
                            ui.label(format!("{titolo}: {:02}:{:02}", secondi / 60, secondi % 60));
                        }
                    }
                });
            }

            let disponibili = !stato.collezioni.is_empty();
            let due_colonne = ui.available_width() >= 760.0;
            if due_colonne {
                // Due pannelli che si spartiscono lo spazio, con una linea
                // in mezzo da trascinare: chi studia decide se vuole piu'
                // PDF o piu' notebook.
                let altezza = ui.available_height();
                let totale = ui.available_width();
                let frazione = stato.config.divisione.clamp(0.2, 0.8);
                let spessore = 10.0;
                let sinistra = (totale * frazione - spessore / 2.0).max(220.0);
                let destra = (totale - sinistra - spessore).max(220.0);

                ui.horizontal_top(|ui| {
                    // La spaziatura fra i tre blocchi la gestisce la linea
                    // centrale: senza azzerarla, i due pannelli sfondano.
                    ui.spacing_mut().item_spacing.x = 0.0;
                    ui.allocate_ui_with_layout(
                        egui::vec2(sinistra, altezza),
                        egui::Layout::top_down(egui::Align::Min),
                        |ui| {
                            // Selettore, poi Zotero a tutta altezza, e il
                            // resto chiuso in fondo.
                            crate::ui_zotero::selettore_esame(ui, stato);
                            let altezzo_pannello = (ui.available_height() - 80.0).max(180.0);
                            crate::ui_zotero::riquadro_zotero(ui, stato, altezzo_pannello);
                            egui::CollapsingHeader::new("⚙ Impostazioni e appelli")
                                .default_open(false)
                                .show(ui, |ui| {
                                    colonna_zotero(ui, stato, audio, mittente, disponibili);
                                    coda(ui, stato);
                                });
                        },
                    );

                    let (rettangolo, risposta) = ui.allocate_exact_size(
                        egui::vec2(spessore, altezza),
                        egui::Sense::drag(),
                    );
                    if risposta.hovered() || risposta.dragged() {
                        ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeHorizontal);
                    }
                    let colore = if risposta.dragged() {
                        ACCENTO
                    } else {
                        crate::ui::BORDO
                    };
                    let x = rettangolo.center().x;
                    ui.painter().line_segment(
                        [
                            egui::pos2(x, rettangolo.top()),
                            egui::pos2(x, rettangolo.bottom()),
                        ],
                        egui::Stroke::new(2.0, colore),
                    );
                    if risposta.dragged() {
                        stato.config.divisione =
                            (frazione + risposta.drag_delta().x / totale).clamp(0.2, 0.8);
                    }
                    if risposta.drag_stopped() {
                        stato.salva_configurazione();
                    }

                    ui.allocate_ui_with_layout(
                        egui::vec2(destra, altezza),
                        egui::Layout::top_down(egui::Align::Min),
                        |ui| {
                            colonna_notebooklm(ui, stato, altezza);
                        },
                    );
                });
            } else {
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        colonna_zotero(ui, stato, audio, mittente, disponibili);
                        colonna_notebooklm(ui, stato, 0.0);
                        coda(ui, stato);
                    });
            }
        });
}

/// Le parti comuni in fondo: appelli e opzioni secondarie.
///
/// La larghezza si calcola qui dentro: fuori e' quella della finestra, ma
/// queste schede vivono in una colonna larga la meta'.
fn coda(ui: &mut egui::Ui, stato: &mut StatoApp) {
    let larghezza = crate::ui::larghezza_schede(ui);
    scheda(ui, larghezza, "📅", "I tuoi appelli", ACCENTO, |ui| {
        crate::ui_appelli::sezione_appelli(ui, stato);
    });

    egui::CollapsingHeader::new("Altre opzioni: archivio, ripasso e storico")
        .default_open(false)
        .show(ui, |ui| {
            scheda(ui, larghezza, "📂", "I tuoi file", TENUE, |ui| crate::ui_archivi::sezione_archivi(ui, stato));
            scheda(ui, larghezza, "🔁", "Ripasso", GIALLO, |ui| crate::ui_ripasso::sezione_ripasso(ui, stato));
            scheda(ui, larghezza, "📈", "Storico", VERDE, |ui| crate::ui_storico::sezione_storico(ui, stato));
        });

    notifica(ui, stato.notifica_attiva());
    avviso(ui, &stato.avviso);
    ui.add_space(10.0);
}

/// Colonna sinistra: obiettivo sessione + esame Zotero + durata e suono.
fn colonna_zotero(
    ui: &mut egui::Ui,
    stato: &mut StatoApp,
    audio: &mut crate::audio::MotoreAudio,
    mittente: &Sender<Segnale>,
    disponibili: bool,
) {
    let l = crate::ui::larghezza_schede(ui);

    scheda(ui, l, "🎯", "Obiettivo", ACCENTO, |ui| {
        sezione_obiettivo(ui, stato, disponibili);
    });

    scheda(ui, l, "🎓", "Il tuo esame", VERDE, |ui| {
        crate::ui_zotero::sezione_esame(ui, stato);
    });

    scheda(ui, l, "⏱", "Durata", ACCENTO, |ui| {
        sezione_durata(ui, stato);
    });

    scheda(ui, l, "🎧", "Suono", ARANCIO, |ui| {
        crate::ui_audio::sezione_audio(ui, stato, audio);
    });
    let _ = mittente;
}

/// Colonna destra: NotebookLM, con la pagina grande e i comandi sotto.
fn colonna_notebooklm(ui: &mut egui::Ui, stato: &mut StatoApp, _altezza_disponibile: f32) {
    let l = crate::ui::larghezza_schede(ui);
    let esame = crate::ui_zotero::nome_esame_corrente(stato);
    scheda(ui, l, "📓", "NotebookLM", VIOLA, |ui| {
        // La pagina prende quasi tutta la colonna: resta solo la riga dei
        // comandi richiusa sotto.
        let altezza_pagina = (ui.available_height() - 130.0).max(150.0);
        crate::ui_notebooklm::sezione_chiedi(ui, stato, &esame, altezza_pagina);
    });
}

/// Intestazione con nome dell'app e stato dei punti.
fn intestazione(ui: &mut egui::Ui, stato: &mut StatoApp) {
    let compatto = stato.config.intestazione_compatta;
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new("🎯").size(if compatto { 20.0 } else { 30.0 }));
        ui.vertical(|ui| {
            ui.label(
                egui::RichText::new("FocusAnchor")
                    .size(if compatto { 18.0 } else { 25.0 })
                    .strong()
                    .color(crate::ui::TESTO),
            );
            if !compatto {
                ui.label(
                    egui::RichText::new("Una sessione alla volta. Il prossimo passo è qui.")
                        .size(13.0)
                        .color(TENUE),
                );
            }
        });
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let (etichetta, aiuto) = if compatto {
                ("▾", "Mostra l'intestazione completa")
            } else {
                ("▴", "Comprimi l'intestazione e lascia spazio allo studio")
            };
            if ui
                .small_button(etichetta)
                .on_hover_text(aiuto)
                .clicked()
            {
                stato.config.intestazione_compatta = !compatto;
                stato.salva_configurazione();
            }
        });
    });

    // Il prossimo appello: l'informazione che decide cosa studiare oggi
    // deve stare senza scorrere.
    let conto = crate::appelli::prossimo(&stato.config)
        .map(|(giorni, esami)| crate::appelli::testo_conto(giorni, &esami));

    if compatto {
        // Una riga sola: XP, grado e countdown.
        ui.horizontal_wrapped(|ui| {
            crate::ui::badge(ui, &format!("⭐ {} XP", stato.xp.punti), VIOLA);
            ui.label(
                egui::RichText::new(stato.xp.grado())
                    .size(12.0)
                    .color(TENUE),
            );
            if let Some(testo) = conto {
                ui.label(egui::RichText::new(testo).size(12.0).color(ARANCIO));
            }
        });
        return;
    }

    ui.add_space(8.0);
    barra_xp(ui, stato);
    if let Some(testo) = conto {
        ui.label(egui::RichText::new(testo).size(13.0).color(ARANCIO));
    }
}

/// Campo di testo con l'obiettivo, oppure la selezione dei testi.
fn sezione_obiettivo(ui: &mut egui::Ui, stato: &mut StatoApp, con_esame: bool) {
    if con_esame {
        // Con un esame scelto l'obiettivo nasce dai testi spuntati.
        let corrente = stato.obiettivo_corrente();
        ui.label(
            egui::RichText::new(if corrente.trim().is_empty() {
                "Scegli i testi qui sotto: l'obiettivo si compila da solo.".to_string()
            } else {
                corrente
            })
            .size(16.0)
            .strong(),
        );
        ui.label(
            egui::RichText::new(
                "Puoi comunque scrivere un obiettivo tuo qui sotto.",
            )
            .size(12.0)
            .color(TENUE),
        );
    }
    egui::TextEdit::singleline(&mut stato.config.obiettivo)
        .hint_text("Esempio: Cap. 3 Storia Romana - Zotero")
        .desired_width(f32::INFINITY)
        .font(egui::TextStyle::Body)
        .show(ui);
    // Niente trim mentre si scrive: togliere gli spazi renderebbe impossibile
    // digitare "Cap. 3 Storia Romana".
}

/// Scelta rapida della durata della sessione.
fn sezione_durata(ui: &mut egui::Ui, stato: &mut StatoApp) {
    ui.horizontal(|ui| {
        for minuti in [25u32, 45, 60] {
            let selezionato = stato.config.durata_minuti == minuti;
            let testo = format!("{minuti} min");
            if ui.selectable_label(selezionato, testo).clicked() {
                stato.config.durata_minuti = minuti;
            }
        }
    });
    ui.add_space(4.0);
    let mut durata = stato.config.durata_minuti;
    // Lo slider ha una larghezza minima che a finestre strette sfonda la
    // scheda: la si adatta allo spazio reale.
    ui.spacing_mut().slider_width = (crate::ui::larghezza(ui) * 0.45).clamp(50.0, 200.0);
    if ui
        .add(egui::Slider::new(&mut durata, 5..=DURATA_MAXIMA).suffix(" min"))
        .changed()
    {
        stato.config.durata_minuti = durata;
    }
}

/// Pulsante di avvio: applica il blocco hosts e apre la sessione.
fn pulsante_avvia(
    ui: &mut egui::Ui,
    stato: &mut StatoApp,
    audio: &mut MotoreAudio,
    mittente: &Sender<Segnale>,
) {
    let pronto = !stato.config.obiettivo.trim().is_empty();
    let colore = if pronto { VERDE } else { crate::ui::BORDO };
    let pulsante = egui::Button::new(
        egui::RichText::new("▶  INIZIA LA SESSIONE")
            .size(18.0)
            .strong()
            .color(if pronto { crate::ui::SFONDO } else { TENUE }),
    )
    .fill(colore)
    .min_size(egui::vec2(crate::ui::larghezza(ui), 50.0));
    let risposta = ui.add_enabled(pronto, pulsante);
    if !pronto {
        ui.label(
            egui::RichText::new("Scrivi l'obiettivo o scegli un testo da Zotero.")
                .size(13.0)
                .color(crate::ui::GIALLO),
        );
        return;
    }
    if risposta.clicked() {
        crate::ui_esami::avvia_sessione(stato, audio, mittente);
    }
}
