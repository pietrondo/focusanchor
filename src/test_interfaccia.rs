//! Prove di robustezza dell'interfaccia: nessuna dimensione deve rompere il
//! layout, e il campo del link NotebookLM non deve perdere il testo.
//!
//! Stanno qui e non in `ui_setup.rs` per non far crescere quel file oltre il
//! limite: sono prove dell'interfaccia intera, non di una singola funzione.

#[cfg(test)]
mod test {
    use crate::state::StatoApp;
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
                    crate::ui_setup::disegna(ctx, &mut stato, &mut audio, &mittente);
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

#[cfg(test)]
mod test_link {
    use crate::state::StatoApp;
    use crate::zotero::Collezione;
    use eframe::egui;

    /// Il campo del link NotebookLM non deve perdere il testo fra un frame e
    /// l'altro: era il difetto per cui l'incolla spariva subito.
    #[test]
    fn il_campo_del_link_mantiene_il_testo() {
        let ctx = egui::Context::default();
        let mut stato = StatoApp::nuovo();
        stato.collezioni = vec![Collezione {
            id: 1,
            nome: "Storia romana".to_string(),
            documenti: 3,
            sottocollezioni: 0,
        }];
        stato.esame_scelto = Some(1);
        let link = "https://notebooklm.google.com/notebook/abc123";
        stato.campo_notebook = link.to_string();

        // Tre frame come in uso reale: il testo deve restare identico.
        for _ in 0..3 {
            let mut inp = egui::RawInput::default();
            inp.screen_rect = Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(620.0, 860.0),
            ));
            let _ = ctx.run(inp, |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    crate::ui_zotero::sezione_esame(ui, &mut stato);
                });
            });
        }
        assert_eq!(stato.campo_notebook, link);
    }
}
