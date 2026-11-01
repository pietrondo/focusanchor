//! Test end-to-end della GUI: verifica che il campo "parcheggio pensieri"
//! salvi davvero il testo quando l'utente preme Invio, e che il codice di
//! sblocco di emergenza resista al copia-incolla.
//!
//! I test guidano l'interfaccia reale di egui (headless) con eventi di
//! tastiera sintetici, quindi non serve aprire una finestra.

use eframe::egui;

/// Testo digitato a mano, un carattere per frame.
const CODICE_MANUALE: &str = "abc";

/// Restituisce `true` se nel frame e' stato premuto il tasto Invio.
fn invio_premuto(ctx: &egui::Context) -> bool {
    ctx.input(|i| i.key_pressed(egui::Key::Enter))
}

/// Restituisce `true` se nel frame e' arrivato un evento di incolla.
fn incolla_rilevata(ctx: &egui::Context) -> bool {
    ctx.input(|i| i.events.iter().any(|e| matches!(e, egui::Event::Paste(_))))
}

/// Costruisce un evento di tastiera per un singolo carattere.
fn tasto(testo: &str) -> egui::Event {
    egui::Event::Text(testo.to_string())
}

/// Esegue un frame di UI dentro il contesto egui.
fn frame(ctx: &egui::Context, mut corpo: impl FnMut(&mut egui::Ui)) {
    let _ = ctx.run(egui::RawInput::default(), |ctx| {
        egui::CentralPanel::default().show(ctx, |ui| {
            corpo(ui);
        });
    });
}

#[test]
fn il_parcheggio_salva_il_testo_alla_pressione_di_invio() {
    let ctx = egui::Context::default();
    let mut testo = String::new();
    let mut salvato: Option<String> = None;

    // Frame 1: l'utente clicca sul campo e prende il focus.
    frame(&ctx, |ui| {
        let risposta = egui::TextEdit::singleline(&mut testo).show(ui);
        risposta.response.request_focus();
    });

    // Frame 2: l'utente digita il testo.
    let mut input = egui::RawInput::default();
    input.events.push(tasto("Chiave di Atena"));
    let _ = ctx.run(input, |ctx| {
        egui::CentralPanel::default().show(ctx, |ui| {
            egui::TextEdit::singleline(&mut testo).show(ui);
        });
    });

    // Frame 3: l'utente preme Invio.
    let mut input = egui::RawInput::default();
    input.events.push(egui::Event::Key {
        key: egui::Key::Enter,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: egui::Modifiers::NONE,
    });
    let _ = ctx.run(input, |ctx| {
        egui::CentralPanel::default().show(ctx, |ui| {
            let risposta = egui::TextEdit::singleline(&mut testo).show(ui);
            if risposta.response.lost_focus() && invio_premuto(ctx) {
                salvato = Some(testo.clone());
            }
        });
    });

    assert_eq!(testo, "Chiave di Atena", "il testo digitato resta nel campo");
    assert_eq!(
        salvato.as_deref(),
        Some("Chiave di Atena"),
        "Invio deve attivare il salvataggio del pensiero"
    );
}

#[test]
fn l_incolla_del_codice_di_sblocco_viene_rifiutato() {
    let ctx = egui::Context::default();
    let mut testo = String::new();
    let mut rifiutato = false;

    // Frame 1: il campo di sblocco prende il focus.
    frame(&ctx, |ui| {
        let risposta = egui::TextEdit::singleline(&mut testo).show(ui);
        risposta.response.request_focus();
    });

    // Frame 2: l'utente incolla il codice invece di digitarlo.
    let mut input = egui::RawInput::default();
    input.events.push(egui::Event::Paste(CODICE_MANUALE.to_string()));
    let _ = ctx.run(input, |ctx| {
        egui::CentralPanel::default().show(ctx, |ui| {
            let risposta = egui::TextEdit::singleline(&mut testo).show(ui);
            if risposta.response.changed() && incolla_rilevata(ctx) {
                testo.clear();
                rifiutato = true;
            }
        });
    });

    assert!(rifiutato, "l'evento di incolla deve essere riconosciuto");
    assert!(
        testo.is_empty(),
        "il testo incollato non deve restare nel campo"
    );
}

#[test]
fn la_digitatura_rapida_non_viene_scambiata_per_incolla() {
    // Tre caratteri in un solo frame: puo' capitare a un utente veloce.
    // Non deve essere scambiato per un copia-incolla.
    let ctx = egui::Context::default();
    let mut testo = String::new();
    let mut rifiutato = false;

    frame(&ctx, |ui| {
        let risposta = egui::TextEdit::singleline(&mut testo).show(ui);
        risposta.response.request_focus();
    });

    let mut input = egui::RawInput::default();
    input.events.push(tasto("abc"));
    let _ = ctx.run(input, |ctx| {
        egui::CentralPanel::default().show(ctx, |ui| {
            let risposta = egui::TextEdit::singleline(&mut testo).show(ui);
            let inseriti: usize = ctx.input(|i| {
                i.events
                    .iter()
                    .filter_map(|e| match e {
                        egui::Event::Text(t) => Some(t.chars().count()),
                        _ => None,
                    })
                    .sum()
            });
            if risposta.response.changed() && (incolla_rilevata(ctx) || inseriti > 12) {
                testo.clear();
                rifiutato = true;
            }
        });
    });

    assert!(!rifiutato, "la digitazione rapida non deve essere rifiutata");
    assert_eq!(testo, "abc", "il testo digitato deve restare nel campo");
}

#[test]
fn il_codice_incollato_in_blocco_viene_rifiutato() {
    // Il codice di sblocco e' lungo 35 caratteri: incollato in un colpo solo
    // supera nettamente la soglia e deve essere rifiutato.
    let ctx = egui::Context::default();
    let mut testo = String::new();
    let mut rifiutato = false;

    frame(&ctx, |ui| {
        let risposta = egui::TextEdit::singleline(&mut testo).show(ui);
        risposta.response.request_focus();
    });

    let codice = "scelgodifermarmiconsapevolmente8";
    let mut input = egui::RawInput::default();
    input.events.push(tasto(codice));
    let _ = ctx.run(input, |ctx| {
        egui::CentralPanel::default().show(ctx, |ui| {
            let risposta = egui::TextEdit::singleline(&mut testo).show(ui);
            let inseriti: usize = ctx.input(|i| {
                i.events
                    .iter()
                    .filter_map(|e| match e {
                        egui::Event::Text(t) => Some(t.chars().count()),
                        _ => None,
                    })
                    .sum()
            });
            if risposta.response.changed() && (incolla_rilevata(ctx) || inseriti > 12) {
                testo.clear();
                rifiutato = true;
            }
        });
    });

    assert!(rifiutato, "il codice incollato in blocco deve essere rifiutato");
    assert!(testo.is_empty(), "il campo deve restare vuoto");
}

#[test]
fn la_digitatura_manuale_del_codice_e_accettata() {
    let ctx = egui::Context::default();
    let mut testo = String::new();

    frame(&ctx, |ui| {
        let risposta = egui::TextEdit::singleline(&mut testo).show(ui);
        risposta.response.request_focus();
    });

    // Un carattere per frame: il comportamento della digitazione manuale.
    for carattere in CODICE_MANUALE.chars() {
        let mut input = egui::RawInput::default();
        input.events.push(tasto(&carattere.to_string()));
        let _ = ctx.run(input, |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                egui::TextEdit::singleline(&mut testo).show(ui);
            });
        });
    }

    assert_eq!(
        testo, CODICE_MANUALE,
        "la digitazione manuale deve riempire il campo"
    );
}
