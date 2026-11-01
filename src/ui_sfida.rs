//! Pop-up di sfida attiva: missione fisica e domanda a tradimento.
//!
//! Quando scatta una sfida, la finestra passa in primo piano e mostra il
//! testo da eseguire o da ricordare. La finestra si chiude solo quando
//! l'utente ha finito: e' il punto dell'intero meccanismo.

use eframe::egui;

use crate::anti_sleep::{Sfida, XP_DOMANDA, XP_MISSIONE};
use crate::archivi;
use crate::audio::MotoreAudio;
use crate::state::StatoApp;
use crate::ui::{avviso, notifica, ARANCIO};

/// Disegna il pop-up della sfida attiva.
pub fn disegna(ctx: &egui::Context, stato: &mut StatoApp, audio: &mut MotoreAudio) {
    // La finestra deve saltare in primo piano: e' il momento di interrompere.
    ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
    ctx.send_viewport_cmd(egui::ViewportCommand::WindowLevel(
        egui::WindowLevel::AlwaysOnTop,
    ));

    egui::CentralPanel::default().show(ctx, |ui| {
        // Il testo della sfida viene copiato prima: serve per passare
        // `stato` in mutabilita' alle funzioni che gestiscono la risposta.
        let testo_sfida = match &stato.anti_sonno.sfida {
            Sfida::Nessuna => None,
            Sfida::Missione { testo, .. } => Some(Sfida::Missione {
                testo: testo.clone(),
                inizio: std::time::Instant::now(),
            }),
            Sfida::Domanda { passaggio, .. } => Some(Sfida::Domanda {
                passaggio: passaggio.clone(),
                inizio: std::time::Instant::now(),
            }),
        };
        match testo_sfida {
            None => {}
            Some(Sfida::Missione { testo, .. }) => {
                disegna_missione(ui, stato, audio, &testo);
            }
            Some(Sfida::Domanda { passaggio, .. }) => {
                disegna_domanda(ui, stato, audio, &passaggio);
            }
            Some(Sfida::Nessuna) => {}
        }
        notifica(ui, stato.notifica_attiva());
        avviso(ui, &stato.avviso);
    });
}

/// Pop-up della micro-missione fisica da 60 secondi.
fn disegna_missione(
    ui: &mut egui::Ui,
    stato: &mut StatoApp,
    audio: &mut MotoreAudio,
    testo: &str,
) {
    ui.label(
        egui::RichText::new("⚡ S.O.S. SONNOLENZA")
            .size(24.0)
            .strong()
            .color(ARANCIO),
    );
    ui.label(
        egui::RichText::new("Il cervello si e' spento. Muoviti per riaccenderlo.")
            .size(15.0),
    );
    ui.add_space(6.0);
    ui.label(egui::RichText::new(testo).size(18.0).strong());
    ui.add_space(6.0);

    let residui = stato.anti_sonno.secondi_residui().unwrap_or(0);
    ui.label(
        egui::RichText::new(format!("⏱ {residui} secondi"))
            .size(20.0)
            .strong(),
    );
    ui.add(
        egui::ProgressBar::new(1.0 - (residui as f32 / 60.0))
            .desired_height(16.0)
            .show_percentage(),
    );
    ui.add_space(8.0);

    if ui
        .add(
            egui::Button::new(egui::RichText::new("✅ Missione Completata!").strong())
                .min_size(egui::vec2(crate::ui::larghezza(ui), 46.0)),
        )
        .clicked()
    {
        stato.aggiungi_xp(XP_MISSIONE);
        stato.anti_sonno.completa_sfida();
        audio.parla(&format!(
            "Bene. Hai guadagnato {XP_MISSIONE} punti. Ora torna al testo."
        ));
        stato.aggiungi_notifica(&format!("+{XP_MISSIONE} XP! Brava, missione completata."));
    }
}

/// Pop-up della domanda a tradimento (active recall).
fn disegna_domanda(
    ui: &mut egui::Ui,
    stato: &mut StatoApp,
    audio: &mut MotoreAudio,
    passaggio: &str,
) {
    ui.label(
        egui::RichText::new("💡 Sfida lampo!")
            .size(24.0)
            .strong()
            .color(ARANCIO),
    );
    ui.label(
        egui::RichText::new("Riassumi in una frase cosa hai appena letto, senza guardare.")
            .size(15.0),
    );
    ui.add_space(6.0);

    // Se il passaggio viene da Zotero, lo si mostra come promemoria.
    if !passaggio.trim().is_empty() {
        ui.label(
            egui::RichText::new("Hai sottolineato questo passaggio su Zotero:")
                .size(14.0)
                .color(egui::Color32::from_rgb(170, 175, 185)),
        );
        ui.label(
            egui::RichText::new(format!("«{}»", passaggio.trim()))
                .size(15.0)
                .italics(),
        );
        ui.add_space(6.0);
    }

    let risposta = egui::TextEdit::multiline(&mut stato.anti_sonno.risposta)
        .hint_text("Scrivi qui cosa hai capito, senza rileggere...")
        .desired_rows(4)
        .desired_width(f32::INFINITY)
        .show(ui);
    let _ = risposta;

    let lunghezza = stato.anti_sonno.risposta.trim().chars().count();
    let minima = crate::anti_sleep::RISPOSTA_MINIMA;
    if lunghezza < minima {
        ui.label(
            egui::RichText::new(format!(
                "Scrivi almeno {minima} caratteri ({lunghezza}/{minima})."
            ))
            .size(13.0)
            .color(egui::Color32::from_rgb(170, 175, 185)),
        );
    }
    // Il tempo che scorre sprona a rispondere di getto, prima di rileggere.
    let trascorsi = stato.anti_sonno.secondi_trascorsi();
    ui.label(
        egui::RichText::new(format!("Ci stai pensando da {trascorsi} secondi."))
            .size(13.0)
            .color(egui::Color32::from_rgb(170, 175, 185)),
    );

    let pronto = stato.anti_sonno.risposta_valida();
    ui.add_space(6.0);
    let esito = ui.add_enabled(
        pronto,
        egui::Button::new(egui::RichText::new("✅ Invia il riassunto").strong())
            .min_size(egui::vec2(crate::ui::larghezza(ui), 44.0)),
    );
    if esito.clicked() {
        completa_domanda(stato, audio);
    }
    if risposta.response.changed() {
        // Il testo e' gia' stato aggiornato nel campo: nessuna azione extra.
    }
}

/// Salva la risposta e assegna i punti XP.
fn completa_domanda(stato: &mut StatoApp, audio: &mut MotoreAudio) {
    let risposta = stato.anti_sonno.risposta.clone();
    let materia = stato.nome_esame();
    // Il passaggio da cui e' nata la domanda diventa la "fronte" della carta.
    let passaggio = match &stato.anti_sonno.sfida {
        Sfida::Domanda { passaggio, .. } => passaggio.clone(),
        _ => String::new(),
    };
    match archivi::salva_riassunto(&materia, &passaggio, &risposta) {
        Ok(()) => {
            stato.aggiungi_xp(XP_DOMANDA);
            stato.anti_sonno.completa_sfida();
            audio.parla(&format!(
                "Ottimo. Hai guadagnato {XP_DOMANDA} punti. Continua cosi."
            ));
            stato.aggiungi_notifica(&format!("+{XP_DOMANDA} XP! Riassunto salvato."));
        }
        Err(e) => stato.avviso = Some(e),
    }
}
