//! Sezione audio della modalita' Configurazione.

use eframe::egui;

use crate::audio::{ModalitaAudio, MotoreAudio};
use crate::state::StatoApp;
use crate::ui::TENUE;

/// Controlli audio: modalita' di sottofondo, volume e velocita' della voce.
pub fn sezione_audio(ui: &mut egui::Ui, stato: &mut StatoApp, audio: &mut MotoreAudio) {
    let modalita = stato.modalita_audio();
    ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new(modalita.etichetta())
                .size(15.0)
                .strong()
                .color(crate::ui::ARANCIO),
        );
    });
    ui.horizontal(|ui| {
        if ui.button("◀ Precedente").clicked() {
            let nuova = modalita.precedente();
            cambia_modalita(stato, audio, nuova);
        }
        if ui.button("Successiva ▶").clicked() {
            let nuova = modalita.successiva();
            cambia_modalita(stato, audio, nuova);
        }
    });
    ui.label(
        egui::RichText::new(match modalita {
            ModalitaAudio::Spenta => "Silenzio: utile se studi in biblioteca.",
            ModalitaAudio::RumoreMarrone => {
                "Rumore marrone: copre i rumori e i pensieri intrusivi."
            }
            ModalitaAudio::Binaurale40 => {
                "Binaurale 40Hz: usa le cuffie, tiene su la vigilanza."
            }
        })
        .size(12.0)
        .color(TENUE),
    );

    let mut volume = stato.config.volume;
    if ui
        .add(
            egui::Slider::new(&mut volume, 0.0..=1.0)
                .text("Volume")
                .custom_formatter(|v, _| format!("{:.0}%", v * 100.0)),
        )
        .changed()
    {
        stato.config.volume = volume;
        audio.imposta_volume(volume);
    }

    let velocita = stato.velocita_voce();
    ui.horizontal(|ui| {
        ui.label("Velocita' voce:");
        if ui.button("◀").clicked() {
            stato.imposta_velocita_voce(velocita.precedente());
        }
        ui.label(egui::RichText::new(velocita.etichetta()).strong());
        if ui.button("▶").clicked() {
            stato.imposta_velocita_voce(velocita.successiva());
        }
        ui.label(
            egui::RichText::new(format!("{}x", velocita.rate()))
                .size(12.0)
                .color(TENUE),
        );
    });

    ui.add_space(2.0);
    ui.checkbox(
        &mut stato.config.richiamo_vocale,
        "Avviso vocale ogni 15 minuti",
    );
    ui.checkbox(
        &mut stato.config.sfide_a_sorpresa,
        "Sfide Anti-Noia a sorpresa",
    );
}

/// Applica una modalita' audio al motore, segnalando gli errori.
pub fn cambia_modalita(stato: &mut StatoApp, audio: &mut MotoreAudio, modalita: ModalitaAudio) {
    stato.imposta_modalita_audio(modalita);
    if let Err(messaggio) = audio.imposta_modalita(modalita, stato.config.volume) {
        stato.avviso = Some(messaggio);
        stato.imposta_modalita_audio(ModalitaAudio::Spenta);
    }
}
