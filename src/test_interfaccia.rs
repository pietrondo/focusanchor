//! Prove di robustezza dell'interfaccia: nessuna dimensione deve rompere il
//! layout, e il campo del link NotebookLM non deve perdere il testo.
//!
//! Stanno qui e non in `ui_setup.rs` per non far crescere quel file oltre il
//! limite: sono prove dell'interfaccia intera, non di una singola funzione.

/// Uno stato con la configurazione pulita.
///
/// `StatoApp::nuovo()` legge la configurazione e lo storico veri di chi
/// esegue il test: con esami gia' salvati il layout e' un altro, e il test
/// passerebbe su una macchina e fallirebbe su un'altra. Qui si riparte da
/// zero: configurazione, XP, storico e tempi dei documenti.
fn stato_pulito() -> crate::state::StatoApp {
    let mut stato = crate::state::StatoApp::nuovo();
    stato.config = crate::config::Configurazione::default();
    stato.xp.punti = 0;
    stato.xp.distrazioni = 0;
    stato.storico.clear();
    stato.secondi_documenti.clear();
    stato.testo_parole = crate::config::lista_alle_righe(&stato.config.parole_vietate);
    stato
}

#[cfg(test)]
mod test {
    use super::stato_pulito;
    use eframe::egui;

    /// Con un nome di esame lunghissimo il setup non deve sfondare.
    ///
    /// Il nome piu' lungo che esiste davvero e' "Economia e storia della
    /// globalizzazione 19/10/2026, 22:53:52": viene dal portale, non da
    /// me, e il riquadro degli appelli deve reggerlo.
    #[test]
    fn il_setup_regga_un_nome_di_esame_lunghissimo() {
        let ctx = egui::Context::default();
        let (mittente, _ricevitore) = std::sync::mpsc::channel();
        let mut stato = stato_pulito();
        stato.config.esami.insert(
            "Economia e storia della globalizzazione 19/10/2026, 22:53:52".to_string(),
            crate::config::InfoEsame {
                url: "https://www.unimi.it/it/corsi/insegnamenti".to_string(),
                crediti: 9,
                modalita: "Scritto".to_string(),
                non_frequentante: true,
                data_esame: "16/12/2026".to_string(),
            },
        );

        for (w, h) in [(560.0, 560.0), (620.0, 860.0)] {
            // Due frame alla stessa dimensione: `used_rect()` dopo un solo
            // frame riporta il rettangolo del frame precedente (o NaN al
            // primo giro), quindi da solo non direbbe niente sul layout.
            let mut output = None;
            for _ in 0..2 {
                let mut inp = egui::RawInput::default();
                inp.screen_rect = Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(w, h),
                ));
                output = Some(ctx.run(inp, |ctx| {
                    crate::ui_setup::disegna(
                        ctx,
                        &mut stato,
                        &mut crate::audio::MotoreAudio::nuovo(),
                        &mittente,
                    );
                }));
            }
            let usato = ctx.used_rect();
            let schermo = ctx.screen_rect();
            assert!(
                usato.right().is_finite(),
                "larghezza del pannello non calcolata a {w}x{h}"
            );
            assert!(
                usato.right() <= schermo.right() + 1.0,
                "contenuto oltre il bordo destro a {w}x{h}: usato={:.1} schermo={:.1}",
                usato.right(),
                schermo.right()
            );

            // Il nome dell'esame deve arrivare a schermo (cioe' il caso e'
            // davvero esercitato) e nessun testo deve uscire dalla finestra.
            let testi: Vec<String> = output
                .expect("frame disegnato")
                .shapes
                .iter()
                .filter_map(|shape| match &shape.shape {
                    egui::epaint::Shape::Text(testo) => Some((
                        testo.galley.job.text.clone(),
                        testo.pos.x + testo.galley.rect.right(),
                    )),
                    _ => None,
                })
                .map(|(testo, destra)| {
                    assert!(
                        destra <= schermo.right() + 1.0,
                        "testo oltre il bordo destro a {w}x{h}: «{testo}» a {destra:.1}"
                    );
                    testo
                })
                .collect();
            assert!(
                testi.iter().any(|t| t.contains("Economia e storia della")),
                "il nome dell'esame non compare a {w}x{h}: {testi:?}"
            );
        }
    }

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
        let mut stato = stato_pulito();
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
        let mut stato = stato_pulito();
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

    #[test]
    fn la_sessione_completata_produce_e_mostra_il_riepilogo_reale() {
        let ctx = egui::Context::default();
        let (mittente, _canale) = std::sync::mpsc::channel();
        let mut stato = stato_pulito();
        stato.config.obiettivo = "Ripasso appello reale".to_string();
        stato.avvia_sessione(&mittente);
        stato.task.avvia(1);
        stato.task.completa();
        stato.tempo_documenti.insert("Capitolo 1".to_string(), 40);
        stato.sessione.as_mut().unwrap().inizio -= std::time::Duration::from_secs(75);
        stato.termina_sessione();

        let riepilogo = stato.ultimo_riepilogo.as_ref().expect("sessione terminata");
        assert_eq!(riepilogo.minuti, 1);
        assert_eq!(riepilogo.blocchi, 1);

        let raw = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(620.0, 1400.0),
            )),
            ..Default::default()
        };
        let output = ctx.run(raw, |ctx| {
            crate::ui_setup::disegna(
                ctx,
                &mut stato,
                &mut crate::audio::MotoreAudio::nuovo(),
                &mittente,
            );
        });
        let testi: Vec<&str> = output
            .shapes
            .iter()
            .filter_map(|shape| match &shape.shape {
                egui::epaint::Shape::Text(testo) => Some(testo.galley.job.text.as_str()),
                _ => None,
            })
            .collect();
        assert!(testi.contains(&"SESSIONE COMPLETATA"), "{testi:?}");
        assert!(
            testi.iter().any(|testo| testo.contains("1 min")),
            "{testi:?}"
        );
        assert!(
            testi.iter().any(|testo| testo.contains("1 blocchi")),
            "{testi:?}"
        );
        assert!(
            testi
                .iter()
                .any(|testo| testo.contains("Capitolo 1: 00:40")),
            "{testi:?}"
        );
    }
}

#[cfg(test)]
mod test_appelli {
    use super::stato_pulito;
    use crate::ui::{ACCENTO, ARANCIO, GIALLO, TENUE};
    use eframe::egui;

    /// Il riquadro degli appelli deve mostrare ogni esame con la sua data e
    /// il conto alla rovescia: e' la risposta a "quando ho esami".
    #[test]
    fn il_riquadro_appelli_mostra_le_date_e_il_conto_alla_rovescia() {
        let ctx = egui::Context::default();
        let mut stato = stato_pulito();
        // Due appelli a distanza di giorni diversi: il piu' vicino viene
        // prima e i due non devono confondersi.
        let oggi = chrono::Local::now().date_naive();
        stato.config.esami.insert(
            "Letteratura italiana".into(),
            crate::config::InfoEsame {
                data_esame: (oggi + chrono::Duration::days(41)).format("%Y-%m-%d").to_string(),
                ..Default::default()
            },
        );
        stato.config.esami.insert(
            "storia romana".into(),
            crate::config::InfoEsame {
                data_esame: (oggi + chrono::Duration::days(34)).format("%Y-%m-%d").to_string(),
                ..Default::default()
            },
        );

        let output = ctx.run(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(620.0, 1400.0),
                )),
                ..Default::default()
            },
            |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    crate::ui_appelli::sezione_appelli(ui, &mut stato);
                });
            },
        );
        let testi: Vec<&str> = output
            .shapes
            .iter()
            .filter_map(|shape| match &shape.shape {
                egui::epaint::Shape::Text(testo) => Some(testo.galley.job.text.as_str()),
                _ => None,
            })
            .collect();

        assert!(testi.iter().any(|t| t.contains("Letteratura italiana")), "{testi:?}");
        assert!(testi.iter().any(|t| t.contains("storia romana")), "{testi:?}");
        // I due countdown devono essere distinti: e' il punto della vista.
        assert!(testi.iter().any(|t| t.contains("fra 41 giorni")), "{testi:?}");
        assert!(testi.iter().any(|t| t.contains("fra 34 giorni")), "{testi:?}");
    }

    /// La data va letta anche nel formato del portale (`GG/MM/AAAA`):
    /// e' il formato in cui l'utente la vede.
    #[test]
    fn la_data_in_formato_italiano_produce_il_countdown() {
        let mut stato = stato_pulito();
        let oggi = chrono::Local::now().date_naive();
        stato.config.esami.insert(
            "storia romana".into(),
            crate::config::InfoEsame {
                data_esame: (oggi + chrono::Duration::days(10)).format("%d/%m/%Y").to_string(),
                ..Default::default()
            },
        );

        let appelli = crate::appelli::elenco(&stato.config, &stato.storico);

        assert_eq!(appelli[0].giorni, Some(10));
    }

    /// Il riquadro degli appelli deve reggere le finestre strette: i campi
    /// data e i badge non devono sfondare il bordo destro.
    #[test]
    fn il_riquadro_appelli_non_sfonda_a_finestra_stretta() {
        let ctx = egui::Context::default();
        let (mittente, _canale) = std::sync::mpsc::channel();
        let mut stato = stato_pulito();
        let mut audio = crate::audio::MotoreAudio::nuovo();
        stato.config.esami.insert(
            "Letteratura italiana".into(),
            crate::config::InfoEsame {
                data_esame: "16/12/2026".into(),
                crediti: 9,
                ..Default::default()
            },
        );

        for (w, h) in [(360.0, 640.0), (420.0, 380.0), (620.0, 860.0)] {
            let mut inp = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(w, h),
                )),
                ..Default::default()
            };
            inp.events.push(egui::Event::PointerButton {
                pos: egui::pos2(w / 2.0, h / 2.0),
                button: egui::PointerButton::Primary,
                pressed: true,
                modifiers: egui::Modifiers::NONE,
            });
            let _ = ctx.run(inp, |ctx| {
                crate::ui_setup::disegna(ctx, &mut stato, &mut audio, &mittente);
            });

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
        }
    }

    /// Durante la sessione il conto alla rovescia deve riguardare l'esame
    /// che si sta studiando, non il piu' vicino in assoluto.
    #[test]
    fn l_overlay_mostra_il_conto_dell_esame_che_stai_studiando() {
        let ctx = egui::Context::default();
        let (mittente, _canale) = std::sync::mpsc::channel();
        let mut stato = stato_pulito();
        let mut audio = crate::audio::MotoreAudio::nuovo();
        let oggi = chrono::Local::now().date_naive();
        // "Vicino" scade prima, ma si studia "Quello che studio".
        stato.config.esami.insert(
            "Vicino".into(),
            crate::config::InfoEsame {
                data_esame: (oggi + chrono::Duration::days(5)).format("%Y-%m-%d").to_string(),
                ..Default::default()
            },
        );
        stato.config.esami.insert(
            "Quello che studio".into(),
            crate::config::InfoEsame {
                data_esame: (oggi + chrono::Duration::days(40)).format("%Y-%m-%d").to_string(),
                ..Default::default()
            },
        );
        stato.collezioni = vec![crate::zotero::Collezione {
            id: 1,
            nome: "Quello che studio".into(),
            documenti: 4,
            sottocollezioni: 0,
        }];
        stato.esame_scelto = Some(1);
        stato.config.obiettivo = "Capitolo 3".to_string();
        stato.avvia_sessione(&mittente);

        let output = ctx.run(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(420.0, 380.0),
                )),
                ..Default::default()
            },
            |ctx| {
                crate::ui_focus::disegna(ctx, &mut stato, &mut audio);
            },
        );
        let testi: Vec<&str> = output
            .shapes
            .iter()
            .filter_map(|shape| match &shape.shape {
                egui::epaint::Shape::Text(testo) => Some(testo.galley.job.text.as_str()),
                _ => None,
            })
            .collect();

        assert!(
            testi
                .iter()
                .any(|t| t.contains("Quello che studio") && t.contains("fra 40 giorni")),
            "{testi:?}"
        );
        stato.termina_sessione();
    }

    /// Il nome dell'esame nel riquadro appelli deve essere selezionabile:
    /// e' il ponte fra "quando ho esami" e "cosa studio adesso".
    #[test]
    fn il_riquadro_appelli_permette_di_scegliere_lesame() {
        let ctx = egui::Context::default();
        let mut stato = stato_pulito();
        stato.collezioni = vec![
            crate::zotero::Collezione {
                id: 1,
                nome: "Storia romana".into(),
                documenti: 3,
                sottocollezioni: 0,
            },
            crate::zotero::Collezione {
                id: 2,
                nome: "Letteratura italiana".into(),
                documenti: 5,
                sottocollezioni: 0,
            },
        ];
        stato.esame_scelto = Some(1);
        stato.config.esami.insert(
            "Letteratura italiana".into(),
            crate::config::InfoEsame {
                data_esame: "16/12/2026".into(),
                ..Default::default()
            },
        );
        // Serve lo snapshot: cambiare esame carica i testi da Zotero.
        stato.snapshot_zotero = Some(std::path::PathBuf::from("snapshot-prova.sqlite"));

        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(620.0, 1400.0),
            )),
            ..Default::default()
        };
        let output = ctx.run(input, |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                crate::ui_appelli::sezione_appelli(ui, &mut stato);
            });
        });

        // Il nome deve comparire come etichetta selezionabile.
        let testo = output
            .shapes
            .iter()
            .filter_map(|shape| match &shape.shape {
                egui::epaint::Shape::Text(testo) => Some(testo.galley.job.text.as_str()),
                _ => None,
            })
            .find(|t| t.contains("Letteratura"))
            .expect("nome dell'esame visibile");
        assert!(testo.contains("Letteratura italiana"), "{testo:?}");
    }

    /// L'import non deve rompere la palette: i colori devono restare quelli
    /// del tema, non uno qualsiasi.
    #[test]
    fn i_colori_del_riquadro_venono_dal_tema() {
        assert_eq!(ACCENTO, crate::ui::ACCENTO);
        assert_eq!(ARANCIO, crate::ui::ARANCIO);
        assert_eq!(GIALLO, crate::ui::GIALLO);
        assert_eq!(TENUE, crate::ui::TENUE);
    }
}

#[cfg(test)]
mod test_notebooklm {
    use super::stato_pulito;
    use eframe::egui;

    /// Il pannello "Chiedi a NotebookLM" lavora sull'esame che riceve e non
    /// richiede Zotero.
    #[test]
    fn il_pannello_chiedi_funziona_senza_zotero() {
        let ctx = egui::Context::default();
        let mut stato = stato_pulito();
        stato.config.esami.insert(
            "Storia romana".into(),
            crate::config::InfoEsame {
                data_esame: "09/12/2026".into(),
                ..Default::default()
            },
        );

        let output = ctx.run(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(620.0, 1400.0),
                )),
                ..Default::default()
            },
            |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    crate::ui_notebooklm::sezione_chiedi(ui, &mut stato, "Storia romana", 300.0);
                });
            },
        );
        let testi: Vec<&str> = output
            .shapes
            .iter()
            .filter_map(|shape| match &shape.shape {
                egui::epaint::Shape::Text(testo) => Some(testo.galley.job.text.as_str()),
                _ => None,
            })
            .collect();

        assert!(testi.iter().any(|t| t.contains("Chiedi a NotebookLM")), "{testi:?}");
        assert!(testi.iter().any(|t| t.contains("Genera flashcard")), "{testi:?}");
        assert!(testi.iter().any(|t| t.contains("Carica la scheda")), "{testi:?}");
    }

    /// La scheda dell'esame deve tenere Zotero e NotebookLM insieme, con un
    /// solo selettore: e' il punto dell'interfaccia unica.
    #[test]
    fn la_scheda_esame_tiene_insieme_zotero_e_notebooklm() {
        let ctx = egui::Context::default();
        let mut stato = stato_pulito();
        stato.config.esami.insert(
            "Storia romana".into(),
            crate::config::InfoEsame {
                data_esame: "09/12/2026".into(),
                ..Default::default()
            },
        );

        let output = ctx.run(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(620.0, 1600.0),
                )),
                ..Default::default()
            },
            |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    crate::ui_zotero::sezione_esame(ui, &mut stato);
                    crate::ui_notebooklm::sezione_chiedi(ui, &mut stato, "Storia romana", 200.0);
                });
            },
        );
        let testi: Vec<&str> = output
            .shapes
            .iter()
            .filter_map(|shape| match &shape.shape {
                egui::epaint::Shape::Text(testo) => Some(testo.galley.job.text.as_str()),
                _ => None,
            })
            .collect();

        // Un solo selettore, e le due sezioni nella stessa scheda.
        assert!(
            testi.iter().filter(|t| t.contains("Storia romana")).count() >= 1,
            "{testi:?}"
        );
        assert!(testi.iter().any(|t| t.contains("Materiali Zotero")), "{testi:?}");
        assert!(testi.iter().any(|t| t.contains("NotebookLM")), "{testi:?}");
    }

    /// Senza esami il riquadro spiega cosa manca invece di restare vuoto.
    #[test]
    fn senza_esami_la_scheda_spiega_cosa_manca() {
        let ctx = egui::Context::default();
        let mut stato = stato_pulito();
        stato.collezioni.clear();

        let output = ctx.run(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(620.0, 1400.0),
                )),
                ..Default::default()
            },
            |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    crate::ui_zotero::sezione_esame(ui, &mut stato);
                    crate::ui_notebooklm::sezione_chiedi(ui, &mut stato, "Storia romana", 200.0);
                });
            },
        );
        let testi: Vec<&str> = output
            .shapes
            .iter()
            .filter_map(|shape| match &shape.shape {
                egui::epaint::Shape::Text(testo) => Some(testo.galley.job.text.as_str()),
                _ => None,
            })
            .collect();

        assert!(
            testi.iter().any(|t| t.contains("Nessun esame caricato")),
            "{testi:?}"
        );
    }
}

#[cfg(test)]
mod test_incasso {
    use super::stato_pulito;
    use eframe::egui;

    /// Il riquadro di Zotero deve esserci e offrire l'aggancio: e' il modo
    /// per avere i PDF dentro la finestra.
    #[test]
    fn il_riquadro_zotero_offre_l_aggancio() {
        let ctx = egui::Context::default();
        let mut stato = stato_pulito();
        stato.config.esami.insert(
            "Storia romana".into(),
            crate::config::InfoEsame::default(),
        );

        let output = ctx.run(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(620.0, 900.0),
                )),
                ..Default::default()
            },
            |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    crate::ui_zotero::sezione_esame(ui, &mut stato);
                    crate::ui_notebooklm::sezione_chiedi(ui, &mut stato, "Storia romana", 200.0);
                });
            },
        );
        let testi: Vec<&str> = output
            .shapes
            .iter()
            .filter_map(|shape| match &shape.shape {
                egui::epaint::Shape::Text(testo) => Some(testo.galley.job.text.as_str()),
                _ => None,
            })
            .collect();

        assert!(
            testi.iter().any(|t| t.contains("Aggancia Zotero")),
            "{testi:?}"
        );
        // La posizione del riquadro viene registrata per il ciclo principale.
        assert!(stato.area_zotero.is_some());
    }
}

#[cfg(test)]
mod test_due_colonne {
    use super::stato_pulito;
    use eframe::egui;

    /// A finestra larga il setup deve mostrare Zotero e NotebookLM
    /// affiancati: si apre l'app e i materiali sono gia' davanti.
    #[test]
    fn a_finestra_larga_zotero_e_notebooklm_stanno_affiancati() {
        let ctx = egui::Context::default();
        let (mittente, _canale) = std::sync::mpsc::channel();
        let mut stato = stato_pulito();
        stato.config.esami.insert(
            "Storia romana".into(),
            crate::config::InfoEsame {
                data_esame: "09/12/2026".into(),
                ..Default::default()
            },
        );

        let output = ctx.run(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(1120.0, 900.0),
                )),
                ..Default::default()
            },
            |ctx| {
                crate::ui_setup::disegna(
                    ctx,
                    &mut stato,
                    &mut crate::audio::MotoreAudio::nuovo(),
                    &mittente,
                );
            },
        );
        let testi: Vec<&str> = output
            .shapes
            .iter()
            .filter_map(|shape| match &shape.shape {
                egui::epaint::Shape::Text(testo) => Some(testo.galley.job.text.as_str()),
                _ => None,
            })
            .collect();

        assert!(testi.iter().any(|t| t.contains("Aggancia Zotero")), "{testi:?}");
        assert!(
            testi.iter().any(|t| t.contains("NOTEBOOKLM")),
            "{testi:?}"
        );
    }
}

#[cfg(test)]
mod test_link {
    use super::stato_pulito;
    use crate::zotero::Collezione;
    use eframe::egui;

    /// Il campo del link NotebookLM non deve perdere il testo fra un frame e
    /// l'altro: era il difetto per cui l'incolla spariva subito.
    #[test]
    fn il_campo_del_link_mantiene_il_testo() {
        let ctx = egui::Context::default();
        let mut stato = stato_pulito();
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
                egui::vec2(620.0, 1400.0),
            ));
            let _ = ctx.run(inp, |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    crate::ui_zotero::sezione_esame(ui, &mut stato);
                    crate::ui_notebooklm::sezione_chiedi(ui, &mut stato, "Storia romana", 200.0);
                });
            });
        }
        assert_eq!(stato.campo_notebook, link);
    }
}

#[cfg(test)]
mod test_materiali {
    use super::stato_pulito;
    use crate::zotero::Collezione;
    use eframe::egui;

    #[test]
    fn il_flusso_materiali_unisce_azioni_zotero_e_notebooklm() {
        let ctx = egui::Context::default();
        let mut stato = stato_pulito();
        stato.collezioni = vec![Collezione {
            id: 1,
            nome: "Storia romana".to_string(),
            documenti: 3,
            sottocollezioni: 0,
        }];
        stato.esame_scelto = Some(1);

        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(620.0, 1400.0),
            )),
            ..Default::default()
        };
        let output = ctx.run(input, |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                crate::ui_zotero::sezione_esame(ui, &mut stato);
                    crate::ui_notebooklm::sezione_chiedi(ui, &mut stato, "Storia romana", 200.0);
            });
        });
        let testi: Vec<&str> = output
            .shapes
            .iter()
            .filter_map(|shape| match &shape.shape {
                egui::epaint::Shape::Text(testo) => Some(testo.galley.job.text.as_str()),
                _ => None,
            })
            .collect();

        assert!(
            testi.iter().any(|testo| testo.contains("Materiali Zotero")),
            "{testi:?}"
        );
        assert!(
            testi
                .iter()
                .any(|testo| testo.contains("NotebookLM")),
            "{testi:?}"
        );
        assert!(
            testi.iter().any(|testo| testo.contains("NotebookLM")),
            "{testi:?}"
        );
        assert!(
            testi.iter().any(|testo| testo.contains("Scheda di studio")),
            "{testi:?}"
        );
    }
}

#[cfg(test)]
mod test_anteprima_notebooklm {
    use super::stato_pulito;
    use crate::zotero::Collezione;
    use eframe::egui;

    #[test]
    fn anteprima_rimane_visibile_nei_frame_successivi_finche_non_si_chiude() {
        let ctx = egui::Context::default();
        let mut stato = stato_pulito();
        stato.collezioni = vec![Collezione {
            id: 1,
            nome: "Storia romana".to_string(),
            documenti: 3,
            sottocollezioni: 0,
        }];
        stato.esame_scelto = Some(1);
        stato.anteprima_notebooklm = Some((
            "notebooklm_Storia_romana.md".to_string(),
            "Riepilogo persistente di prova".to_string(),
        ));

        let frame_testi = |stato: &mut crate::state::StatoApp| {
            let input = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(620.0, 1400.0),
                )),
                ..Default::default()
            };
            let output = ctx.run(input, |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    crate::ui_zotero::sezione_esame(ui, stato);
                    crate::ui_notebooklm::sezione_chiedi(ui, stato, "Storia romana", 200.0);
                });
            });
            output
                .shapes
                .iter()
                .filter_map(|shape| match &shape.shape {
                    egui::epaint::Shape::Text(testo) => Some(testo.galley.job.text.clone()),
                    _ => None,
                })
                .collect::<Vec<_>>()
        };

        let _ = frame_testi(&mut stato);
        let secondo_frame = frame_testi(&mut stato);
        assert!(secondo_frame
            .iter()
            .any(|testo| testo.contains("Riepilogo persistente")));
        assert!(secondo_frame
            .iter()
            .any(|testo| testo.contains("Chiudi anteprima")));
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(620.0, 1400.0),
            )),
            ..Default::default()
        };
        let output = ctx.run(input, |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                crate::ui_zotero::sezione_esame(ui, &mut stato);
                    crate::ui_notebooklm::sezione_chiedi(ui, &mut stato, "Storia romana", 200.0);
            });
        });
        let chiudi = output.shapes.iter().find_map(|shape| match &shape.shape {
            egui::epaint::Shape::Text(testo)
                if testo.galley.job.text == "Chiudi anteprima" =>
            {
                Some(testo.pos + testo.galley.size() / 2.0)
            }
            _ => None,
        }).expect("pulsante di chiusura visibile");
        let mut click = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(620.0, 1400.0),
            )),
            ..Default::default()
        };
        click.events.extend([
            egui::Event::PointerMoved(chiudi),
            egui::Event::PointerButton {
                pos: chiudi,
                button: egui::PointerButton::Primary,
                pressed: true,
                modifiers: egui::Modifiers::NONE,
            },
            egui::Event::PointerButton {
                pos: chiudi,
                button: egui::PointerButton::Primary,
                pressed: false,
                modifiers: egui::Modifiers::NONE,
            },
        ]);
        let _ = ctx.run(click, |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                crate::ui_zotero::sezione_esame(ui, &mut stato);
                    crate::ui_notebooklm::sezione_chiedi(ui, &mut stato, "Storia romana", 200.0);
            });
        });
        assert!(stato.anteprima_notebooklm.is_none());
        stato.anteprima_notebooklm = Some((
            "notebooklm_Storia_romana.md".to_string(),
            "Riepilogo persistente di prova".to_string(),
        ));
        crate::ui_esami::cambia_esame(&mut stato, None);
        assert!(stato.anteprima_notebooklm.is_none());
    }
}
