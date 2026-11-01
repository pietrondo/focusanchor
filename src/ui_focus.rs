//! Modalita' Focus Attivo: overlay compatto sempre in primo piano.

use eframe::egui;

use crate::anti_sleep::Sfida;
use crate::audio::MotoreAudio;
use crate::state::StatoApp;
use crate::ui::{avviso, badge, notifica, ACCENTO, ARANCIO, GIALLO, TENUE, VERDE, VIOLA};

/// Disegna l'overlay compatto della sessione attiva.
pub fn disegna(ctx: &egui::Context, stato: &mut StatoApp, audio: &mut MotoreAudio) {
    // Le sfide hanno la precedenza: devono interrompere.
    if !matches!(stato.anti_sonno.sfida, Sfida::Nessuna) {
        crate::ui_sfida::disegna(ctx, stato, audio);
        return;
    }

    egui::CentralPanel::default()
        .frame(
            egui::Frame::none()
                .fill(crate::ui::SFONDO)
                .inner_margin(egui::Margin::symmetric(14.0, 10.0)),
        )
        .show(ctx, |ui| {
            intestazione(ui, stato);
            ui.add_space(2.0);
            countdown(ui, stato);
            pannello_pausa(ui, stato);
            ui.add_space(6.0);

            // La larghezza si ricava dalla finestra vera: `available_width`
            // dentro i pannelli annidati non e' affidabile e i pulsanti
            // finivano fuori dal bordo destro.
            let larghezza = (ctx.screen_rect().width() - 2.0 * 14.0 - 12.0).max(160.0);

            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    if stato.interleaving.attivo() {
                        pannello_interleaving(ui, stato);
                        ui.separator();
                    }

                    crate::ui_azioni::disegna(ui, stato, audio, larghezza);
                    ui.add_space(4.0);

                    crate::ui_ai::mostra_risposta(ui, stato);

                    pannello_task(ui, stato, audio);
                    ui.add_space(4.0);

                    parcheggio_pensieri(ui, stato);
                    ui.add_space(6.0);

                    crate::ui_sblocco::disegna(ui, stato, audio, ctx);

                    notifica(ui, stato.notifica_attiva());
                    avviso(ui, &stato.avviso);
                    ui.add_space(6.0);
                });
        });
}

/// Obiettivo in evidenza e stato dei punti.
fn intestazione(ui: &mut egui::Ui, stato: &StatoApp) {
    ui.label(
        egui::RichText::new(format!("🎯 {}", stato.obiettivo_corrente()))
            .size(17.0)
            .strong(),
    );
    ui.horizontal(|ui| {
        badge(ui, &format!("⭐ {}", stato.xp.punti), VIOLA);
        badge(ui, &stato.xp.grado().to_string(), TENUE);
        if stato.xp.distrazioni > 0 {
            badge(ui, &format!("⛔ {}", stato.xp.distrazioni), GIALLO);
        }
    });
    // Quanto manca all'appello piu' vicino: cambia il peso di ogni sessione.
    if let Some((esame, giorni)) = crate::ui_storico::prossimo_esame(stato) {
        let testo = if giorni == 0 {
            format!("📅 {esame}: oggi")
        } else {
            format!("📅 {esame}: {} giorni", giorni)
        };
        ui.label(egui::RichText::new(testo).size(13.0).color(ARANCIO));
    }
}

/// Countdown in grande e barra di avanzamento.
fn countdown(ui: &mut egui::Ui, stato: &StatoApp) {
    let residuo = stato.tempo_residuo();
    let minuti = residuo.as_secs() / 60;
    let secondi = residuo.as_secs() % 60;
    ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new(format!("{minuti:02}:{secondi:02}"))
                .size(38.0)
                .strong(),
        );
        if stato.in_pausa() {
            badge(ui, "IN PAUSA", ARANCIO);
        }
    });
    ui.add(
        egui::ProgressBar::new(stato.frazione_transcorsa())
            .desired_height(14.0)
            .rounding(egui::Rounding::same(7.0))
            .fill(ACCENTO)
            .show_percentage(),
    );
}

/// Pulsante pausa e conto alla rovescia della pausa.
///
/// Una pausa pianificata e' ciò che tiene in piedi una sessione lunga: senza,
/// l'unico modo per fermarsi sarebbe lo sblocco d'emergenza.
fn pannello_pausa(ui: &mut egui::Ui, stato: &mut StatoApp) {
    match stato.pausa_residua() {
        Some(secondi) => {
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new(format!(
                        "☕ Pausa: {:02}:{:02}",
                        secondi / 60,
                        secondi % 60
                    ))
                    .size(15.0)
                    .color(ARANCIO),
                );
                if ui.button("▶ Riprendi").clicked() {
                    stato.riprendi_sessione();
                    stato.aggiungi_notifica("Si riparte.");
                }
            });
        }
        None => {
            let larghezza = crate::ui::larghezza(ui);
            if ui
                .add(
                    egui::Button::new("☕ Pausa 5 minuti")
                        .min_size(egui::vec2(larghezza, 26.0)),
                )
                .on_hover_text("Ferma il timer: nessuna penalità, si riprende da soli")
                .clicked()
            {
                stato.metti_in_pausa();
                stato.aggiungi_notifica("Pausa di 5 minuti. Il timer è fermo.");
            }
        }
    }
}

/// Riga dell'alternanza: testo corrente e tempo al cambio.
fn pannello_interleaving(ui: &mut egui::Ui, stato: &mut StatoApp) {
    if stato.in_pausa() {
        ui.label(
            egui::RichText::new("Tempo scaduto su questo testo. Respira, poi riparti.")
                .size(14.0)
                .color(ARANCIO),
        );
        if ui
            .add(
                egui::Button::new(
                    egui::RichText::new("▶  Riprendi sul testo nuovo").strong(),
                )
                .min_size(egui::vec2(crate::ui::larghezza(ui), 36.0)),
            )
            .clicked()
        {
            stato.riprendi_dopo_cambio();
        }
        return;
    }
    match stato.interleaving.secondi_residui() {
        Some(residui) => {
            ui.label(
                egui::RichText::new(format!(
                    "Cambio testo fra {:02}:{:02}",
                    residui / 60,
                    residui % 60
                ))
                .size(14.0)
                .color(VERDE),
            );
            ui.add(
                egui::ProgressBar::new(stato.interleaving.frazione())
                    .desired_height(8.0)
                    .fill(VERDE),
            );
        }
        None => {
            ui.label(egui::RichText::new("Un solo testo selezionato.").color(TENUE));
        }
    }
}

/// Pannello dei micro-task: un blocco breve, un premio subito.
fn pannello_task(ui: &mut egui::Ui, stato: &mut StatoApp, audio: &mut MotoreAudio) {
    if let Some(residui) = stato.task.secondi_residui() {
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new(format!("⏳ {:02}:{:02}", residui / 60, residui % 60))
                    .size(20.0)
                    .strong()
                    .color(VERDE),
            );
            if ui.button("✋ Salta").clicked() {
                stato.task.annulla();
                stato.aggiungi_notifica("Blocco saltato. Ripartiamo.");
            }
        });
        ui.add(
            egui::ProgressBar::new(stato.task.frazione())
                .desired_height(10.0)
                .fill(VERDE),
        );
        return;
    }

    if stato.task.e_completato() {
        let premio = stato.task.completa().unwrap_or(0);
        stato.aggiungi_xp(premio);
        // Solo un beep: un annuncio vocale a ogni blocco diventa rumore.
        audio.beep();
        stato.aggiungi_notifica(&format!("+{premio} XP! Blocco completato."));
        return;
    }

    ui.horizontal_wrapped(|ui| {
        for minuti in crate::task::DURATE {
            if ui
                .button(crate::task::etichetta(minuti))
                .on_hover_text(format!("Premio: +{} XP", crate::task::premio_per(minuti)))
                .clicked()
            {
                stato.task.avvia(minuti);
            }
        }
    });
}

/// Campo "parcheggio pensieri" sempre a portata di mano.
fn parcheggio_pensieri(ui: &mut egui::Ui, stato: &mut StatoApp) {
    let risposta = egui::TextEdit::singleline(&mut stato.campo_pensiero)
        .hint_text("Dubbio improvviso? Scrivi e premi Invio")
        .desired_width(f32::INFINITY)
        .show(ui);
    // In una TextEdit su una riga il tasto Invio fa perdere il focus al widget:
    // e' esattamente il comportamento atteso per confermare il salvataggio.
    if risposta.response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
        stato.salva_pensiero();
    }
}
