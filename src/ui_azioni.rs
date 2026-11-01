//! Azioni rapide dell'overlay: lettura, NotebookLM, S.O.S. e audio.

use eframe::egui;

use crate::audio::MotoreAudio;
use crate::state::StatoApp;

/// Pulsanti rapidi sempre a portata di mano durante lo studio.
pub fn disegna(ui: &mut egui::Ui, stato: &mut StatoApp, audio: &mut MotoreAudio) {
    ui.horizontal(|ui| {
        if ui
            .button("🔊 Leggi da Zotero")
            .on_hover_text("Legge il testo copiato con Ctrl+C")
            .clicked()
        {
            leggi_testo_copiato(stato, audio);
        }
        if ui
            .button("📤 Spiega con NotebookLM")
            .on_hover_text("Copia una domanda pronta: incollala nella chat di NotebookLM")
            .clicked()
        {
            spiega_con_notebooklm(stato);
        }
    });
    ui.horizontal(|ui| {
        if ui
            .button("⚡ S.O.S. Sonnolenza")
            .on_hover_text("Reset di 60 secondi: ti alzi e ti muovi")
            .clicked()
        {
            let missione = stato.anti_sonno.avvia_missione();
            audio.parla(&format!("Missione di un minuto. {missione}"));
        }
        if ui.button("⏹ Ferma voce").clicked() {
            audio.ferma_voce();
        }
    });
    ui.horizontal(|ui| {
        if ui.button("▶ Apri NotebookLM").clicked() {
            if let Err(e) = crate::notebooklm::apri_notebooklm() {
                stato.avviso = Some(e);
            }
        }
        let modalita = stato.modalita_audio();
        if ui
            .button(format!("🎵 {}", modalita.etichetta()))
            .on_hover_text("Cambia il suono di sottofondo")
            .clicked()
        {
            let nuova = modalita.successiva();
            crate::ui_audio::cambia_modalita(stato, audio, nuova);
        }
    });
}

/// Legge il testo copiato dagli appunti e lo pronuncia.
fn leggi_testo_copiato(stato: &mut StatoApp, audio: &mut MotoreAudio) {
    match MotoreAudio::leggi_clipboard() {
        Ok(testo) if testo.trim().is_empty() => {
            stato.avviso = Some(
                "Il testo copiato e' vuoto. Seleziona il paragrafo su Zotero e premi Ctrl+C."
                    .into(),
            );
        }
        Ok(testo) => {
            let velocita = stato.velocita_voce();
            audio.parla_con_velocita(&testo, velocita);
        }
        Err(errore) => {
            stato.avviso = Some(format!("Non riesco a leggere il testo copiato: {errore}"));
        }
    }
}

/// Prepara il messaggio da incollare nella chat di NotebookLM.
///
/// E' il passo che l'utente fa gia' a mano: copia un passaggio e chiede
/// "spiegami meglio". Qui il messaggio viene composto con l'esame giusto e
/// messo negli appunti, pronto da incollare.
fn spiega_con_notebooklm(stato: &mut StatoApp) {
    let passaggio = match MotoreAudio::leggi_clipboard() {
        Ok(t) if !t.trim().is_empty() => t,
        Ok(_) => {
            stato.avviso = Some(
                "Copia prima il passaggio da spiegare (Ctrl+C), poi premi di nuovo.".into(),
            );
            return;
        }
        Err(e) => {
            stato.avviso = Some(format!("Appunti non leggibili: {e}"));
            return;
        }
    };
    let messaggio = crate::notebooklm::prepara_messaggio(&stato.nome_esame(), &passaggio);
    match crate::notebooklm::copia_negli_appunti(&messaggio) {
        Ok(()) => {
            stato.aggiungi_notifica("Domanda copiata: incollala su NotebookLM e premi Invio.");
            let _ = crate::notebooklm::apri_notebooklm();
        }
        Err(e) => stato.avviso = Some(e),
    }
}
