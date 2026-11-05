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
    ui.horizontal_wrapped(|ui| {
        for scelta in [
            ModalitaAudio::Spenta,
            ModalitaAudio::RumoreMarrone,
            ModalitaAudio::Pioggia,
            ModalitaAudio::PioggiaFile,
            ModalitaAudio::PioggiaCitta,
            ModalitaAudio::PioggiaDelicata,
            ModalitaAudio::Binaurale4,
            ModalitaAudio::Binaurale40,
        ] {
            if ui
                .selectable_label(modalita == scelta, scelta.etichetta())
                .clicked()
            {
                cambia_modalita(stato, audio, scelta);
            }
        }
    });
    ui.label(
        egui::RichText::new(match modalita {
            ModalitaAudio::Spenta => "Silenzio: utile se studi in biblioteca.",
            ModalitaAudio::RumoreMarrone => {
                "Rumore marrone: copre i rumori e i pensieri intrusivi."
            }
            ModalitaAudio::Pioggia => "Pioggia: copre come il rumore, ma suona come pioggia.",
            ModalitaAudio::PioggiaFile => {
                "Pioggia naturale: legge pioggia.mp3 dalla cartella dati dell'app. Sostituisci il file per cambiarla."
            }
            ModalitaAudio::PioggiaCitta => "Registrazione CC0: pioggia urbana, circa 2 minuti e 30 secondi; ripetuta in ciclo.",
            ModalitaAudio::PioggiaDelicata => "Registrazione CC0: pioggia delicata, circa 5 minuti; qualità preview ridotta, ripetuta in ciclo.",
            ModalitaAudio::Binaurale40 => "Toni binaurali con differenza di 40 Hz. Nessun beneficio attentivo è garantito.",
            ModalitaAudio::Binaurale4 => "Toni binaurali con differenza di 4 Hz. Nessun beneficio attentivo è garantito.",
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
    sezione_voce(ui, stato);
    sezione_parole(ui, stato);
}

/// Interruttori della voce e degli avvisi.
fn sezione_voce(ui: &mut egui::Ui, stato: &mut StatoApp) {
    ui.checkbox(&mut stato.config.voce_attiva, "Voce attiva");
    ui.label(
        egui::RichText::new(
            "Se senti delle voci sopra il rumore sono gli avvisi vocali: spegnili qui.",
        )
        .size(12.0)
        .color(TENUE),
    );
    ui.checkbox(
        &mut stato.config.richiamo_vocale,
        "Avviso vocale ogni 15 minuti",
    );
    ui.checkbox(
        &mut stato.config.sfide_a_sorpresa,
        "Sfide Anti-Noia a sorpresa",
    );
}

/// Parole che fanno perdere il filo: servono solo agli avvisi.
fn sezione_parole(ui: &mut egui::Ui, stato: &mut StatoApp) {
    egui::CollapsingHeader::new("🔔 Parole che fanno perdere il filo")
        .default_open(false)
        .show(ui, |ui| {
            ui.label(
                egui::RichText::new(
                    "Quando una finestra con queste parole va in primo piano, l'app te lo ricorda. \
                     Non chiude e non blocca niente: una parola per riga.",
                )
                .size(12.0)
                .color(TENUE),
            );
            egui::TextEdit::multiline(&mut stato.testo_parole)
                .desired_rows(5)
                .desired_width(f32::INFINITY)
                .show(ui);
            if ui.button("💾 Salva").clicked() {
                stato.salva_configurazione();
                stato.aggiungi_notifica("Parole salvate.");
            }
        });
}

/// Applica una modalita' audio al motore, segnalando gli errori.
pub fn cambia_modalita(stato: &mut StatoApp, audio: &mut MotoreAudio, modalita: ModalitaAudio) {
    stato.imposta_modalita_audio(modalita);
    if let Err(messaggio) = audio.imposta_modalita(modalita, stato.config.volume) {
        stato.avviso = Some(messaggio);
        stato.imposta_modalita_audio(ModalitaAudio::Spenta);
    }
}
