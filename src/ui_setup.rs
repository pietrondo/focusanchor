//! Modalita' Configurazione: esame, testi, durata e audio.

use std::sync::mpsc::Sender;

use eframe::egui;

use crate::audio::MotoreAudio;
use crate::blocker::Segnale;
use crate::state::{StatoApp, DURATA_MAXIMA};
use crate::ui::{avviso, barra_xp, notifica, scheda, ACCENTO, ARANCIO, TENUE, VERDE};

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

            // La larghezza si calcola qui fuori: dentro lo ScrollArea
            // potrebbe essere infinita.
            let l = crate::ui::larghezza_schede(ui);
            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    let disponibili = !stato.collezioni.is_empty();
                    scheda(ui, l, "🎯", "Obiettivo", ACCENTO, |ui| {
                        sezione_obiettivo(ui, stato, disponibili);
                    });

                    scheda(ui, l, "📚", "Esame e testi", VERDE, |ui| {
                        crate::ui_zotero::sezione_esame(ui, stato);
                    });

                    scheda(ui, l, "⏱", "Durata", ACCENTO, |ui| {
                        sezione_durata(ui, stato);
                    });

                    scheda(ui, l, "🎧", "Audio", ARANCIO, |ui| {
                        crate::ui_audio::sezione_audio(ui, stato, audio);
                    });

                    scheda(ui, l, "📂", "I tuoi file", TENUE, |ui| {
                        crate::ui_archivi::sezione_archivi(ui, stato);
                    });

                    ui.add_space(4.0);
                    pulsante_avvia(ui, stato, audio, mittente);

                    notifica(ui, stato.notifica_attiva());
                    avviso(ui, &stato.avviso);
                    ui.add_space(10.0);
                });
        });
}

/// Intestazione con nome dell'app e stato dei punti.
fn intestazione(ui: &mut egui::Ui, stato: &StatoApp) {
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new("🎯").size(30.0));
        ui.vertical(|ui| {
            ui.label(
                egui::RichText::new("FocusAnchor")
                    .size(25.0)
                    .strong()
                    .color(crate::ui::TESTO),
            );
            ui.label(
                egui::RichText::new("Un obiettivo alla volta. Cinque esami. Dicembre.")
                    .size(13.0)
                    .color(TENUE),
            );
        });
    });
    ui.add_space(8.0);
    barra_xp(ui, stato);
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

#[cfg(test)]
mod test {
    use super::*;
    use eframe::egui;

    /// Costruisce un input con una certa dimensione e il puntatore in un punto.
    fn input(larghezza: f32, altezza: f32, pos: egui::Pos2) -> egui::RawInput {
        let mut i = egui::RawInput::default();
        i.screen_rect = Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(larghezza, altezza),
        ));
        i.events.push(egui::Event::PointerMoved(pos));
        i
    }

    /// La schermata di setup non deve andare in panic a nessuna dimensione.
    ///
    /// Il panic che si vuole evitare e' in `egui::hit_test`: se una coordinata
    /// diventa NaN, il confronto fra rect fallisce e `unwrap` esplode.
    #[test]
    fn il_setup_non_esplode_a_nessuna_dimensione() {
        let ctx = egui::Context::default();
        let (mittente, _canale) = std::sync::mpsc::channel();
        let mut stato = StatoApp::nuovo();
        let mut audio = crate::audio::MotoreAudio::nuovo();

        // Dimensioni realistiche e non: la finestra puo' essere ridotta,
        // e l'overlay e' molto piu' piccolo della schermata di setup.
        let misure = [
            // La finestra non scende sotto 560x560; l'overlay e' 420x380.
            (420.0, 380.0),
            (560.0, 560.0),
            (634.0, 898.0),
            (900.0, 700.0),
            (1200.0, 900.0),
        ];
        for (w, h) in misure {
            for pos in [
                egui::pos2(0.0, 0.0),
                egui::pos2(w / 2.0, h / 2.0),
                egui::pos2(w - 1.0, h - 1.0),
                egui::pos2(40.0, 120.0),
            ] {
                let mut inp = input(w, h, pos);
                // Un click, per esercitare anche il percorso di "hit" completo.
                inp.events.push(egui::Event::PointerButton {
                    pos,
                    button: egui::PointerButton::Primary,
                    pressed: true,
                    modifiers: egui::Modifiers::NONE,
                });
                let _ = ctx.run(inp, |ctx| {
                    disegna(ctx, &mut stato, &mut audio, &mittente);
                });

                // Nessun contenuto deve uscire dalla finestra: era il difetto
                // per cui le schede venivano tagliate a destra.
                // I valori non finiti li calcola egui per conto suo e non
                // arrivano a schermo: qui interessa lo sfondamento reale.
                let usato = ctx.used_rect();
                let schermo = ctx.screen_rect();
                if usato.right().is_finite() {
                    assert!(
                        usato.right() <= schermo.right() + 1.0,
                        "contenuto oltre il bordo destro a {w}x{h}: usato={:.1} schermo={:.1}",
                        usato.right(),
                        schermo.right()
                    );
                }
                if usato.left().is_finite() {
                    assert!(
                        usato.left() >= schermo.left() - 1.0,
                        "contenuto oltre il bordo sinistro a {w}x{h}"
                    );
                }
            }
        }
    }

    /// Anche l'overlay di focus deve reggere le stesse sollecitazioni.
    #[test]
    fn l_overlay_non_esplode_a_nessuna_dimensione() {
        let ctx = egui::Context::default();
        let (mittente, _canale) = std::sync::mpsc::channel();
        let mut stato = StatoApp::nuovo();
        let mut audio = crate::audio::MotoreAudio::nuovo();
        stato.config.obiettivo = "Prova".to_string();
        stato.avvia_sessione(&mittente);

        for (w, h) in [(200.0, 200.0), (420.0, 380.0), (900.0, 700.0)] {
            for pos in [
                egui::pos2(1.0, 1.0),
                egui::pos2(w / 2.0, h / 2.0),
                egui::pos2(w - 1.0, h - 1.0),
            ] {
                let mut inp = input(w, h, pos);
                inp.events.push(egui::Event::PointerButton {
                    pos,
                    button: egui::PointerButton::Primary,
                    pressed: true,
                    modifiers: egui::Modifiers::NONE,
                });
                let _ = ctx.run(inp, |ctx| {
                    crate::ui_focus::disegna(ctx, &mut stato, &mut audio);
                });
            }
        }
        stato.termina_sessione();
    }
}
