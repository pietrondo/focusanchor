//! Azioni rapide dell'overlay: lettura, NotebookLM, S.O.S. e audio.

use eframe::egui;

use crate::audio::MotoreAudio;
use crate::state::StatoApp;

/// Pulsanti rapidi sempre a portata di mano durante lo studio.
///
/// Un pulsante per riga, a tutta larghezza: con le emoji la larghezza del
/// testo e' imprevedibile e due colonne sfondavano la finestra.
pub fn disegna(
    ui: &mut egui::Ui,
    stato: &mut StatoApp,
    audio: &mut MotoreAudio,
    larghezza: f32,
) {
    let misura = egui::vec2(larghezza, 30.0);

    if ui
        .add(egui::Button::new("🔊 Leggi il testo copiato").min_size(misura))
        .on_hover_text("Legge ad alta voce quello che hai copiato con Ctrl+C")
        .clicked()
    {
        leggi_testo_copiato(stato, audio);
    }

    if ui
        .add(egui::Button::new("📤 Spiega con NotebookLM").min_size(misura))
        .on_hover_text("Prepara la domanda e apre il notebook dell'esame")
        .clicked()
    {
        spiega_con_notebooklm(stato);
    }

    let etichetta_ai = if stato.ai.in_corso {
        "🤖 Sto pensando…"
    } else {
        "🤖 Spiegami questo passaggio"
    };
    if ui
        .add(egui::Button::new(etichetta_ai).min_size(misura))
        .on_hover_text("Chiede al modello di spiegare il testo copiato, nel contesto dell'esame")
        .clicked()
    {
        chiedi_all_ai(stato);
    }

    if ui
        .add(egui::Button::new("⚡ S.O.S. Sonnolenza").min_size(misura))
        .on_hover_text("Reset di 60 secondi: ti alzi e ti muovi")
        .clicked()
    {
        let missione = stato.anti_sonno.avvia_missione();
        audio.parla(&format!("Missione di un minuto. {missione}"));
    }

    ui.horizontal(|ui| {
        let meta = ((larghezza - 8.0) / 2.0).max(70.0);
        if ui
            .add(egui::Button::new("⏹ Ferma voce").min_size(egui::vec2(meta, 28.0)))
            .clicked()
        {
            audio.ferma_voce();
        }
        let modalita = stato.modalita_audio();
        let etichetta = match modalita {
            crate::audio::ModalitaAudio::Spenta => "🎵 Spento",
            crate::audio::ModalitaAudio::RumoreMarrone => "🎵 Marrone",
            crate::audio::ModalitaAudio::Binaurale40 => "🎵 40Hz",
        };
        if ui
            .add(egui::Button::new(etichetta).min_size(egui::vec2(meta, 28.0)))
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
/// "spiegami meglio". Qui il messaggio viene composto con l'esame giusto,
/// messo negli appunti, e si apre il notebook di quell'esame se il link e'
/// stato salvato.
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

    let esame = stato.nome_esame();
    let messaggio = crate::notebooklm::prepara_messaggio(&esame, &passaggio);
    match crate::notebooklm::copia_negli_appunti(&messaggio) {
        Ok(()) => {
            let link = stato
                .config
                .notebook_esami
                .get(&esame)
                .filter(|l| !l.trim().is_empty())
                .cloned();
            match link {
                Some(link) => {
                    let _ = crate::notebooklm::apri(&link);
                    stato.aggiungi_notifica("Domanda copiata: incollala e premi Invio.");
                }
                None => stato.aggiungi_notifica(
                    "Domanda copiata: incollala nel notebook dell'esame su NotebookLM.",
                ),
            }
        }
        Err(e) => stato.avviso = Some(e),
    }
}

/// Chiede all'assistente AI di spiegare il passaggio copiato.
///
/// Riusa lo stesso testo che l'utente copia per NotebookLM: cambia solo il
/// destinatario. La risposta arriva in un thread e compare nell'overlay.
fn chiedi_all_ai(stato: &mut StatoApp) {
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
    let esame = stato.nome_esame();
    let modello = stato.config.modello_ai.clone();
    // Le sottolineature dell'esame vengono da Zotero: danno al modello il
    // taglio del corso, non solo il paragrafo copiato.
    let contesto: Vec<String> = stato
        .evidenziazioni
        .iter()
        .map(|e| e.testo.clone())
        .collect();
    stato.ai.chiedi(&modello, &esame, &passaggio, &contesto);
}
