//! Sblocco di emergenza: fermare la sessione costa attrito.
//!
//! L'idea e' semplice: per interrompere prima del tempo bisogna digitare a
//! mano un codice casuale. Il copia-incolla e' bloccato. Non e' una punizione,
//! e' un secondo di tempo per chiedersi se fermarsi serve davvero.

use eframe::egui;
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};

use crate::audio::MotoreAudio;
use crate::state::StatoApp;
use crate::ui::GIALLO;

/// Lunghezza del codice da digitare.
pub const LUNGHEZZA_CODICE: usize = 35;

/// Numero massimo di caratteri inseriti in un solo frame.
///
/// La rilevazione principale del copia-incolla e' l'evento `Paste` di egui,
/// che e' certo. Questo valore e' la rete di sicurezza per gli incolla che
/// arrivano senza evento esplicito (IME, trascinamento).
///
/// Il limite non puo' essere 1-2 come suggerirebbe la specifica: un utente
/// veloce che digita 3 caratteri in un frame verrebbe bloccato per sempre,
/// proprio l'utente che piu' ne ha bisogno.
pub const MAX_CARATTERI_PER_FRAME: usize = 12;

/// Disegna il blocco dello sblocco di emergenza.
pub fn disegna(
    ui: &mut egui::Ui,
    stato: &mut StatoApp,
    audio: &mut MotoreAudio,
    ctx: &egui::Context,
) {
    if stato.codice_sblocco.is_none() {
        if ui
            .add(
                egui::Button::new(egui::RichText::new("⏻ Sblocco di emergenza").size(13.0))
                    .min_size(egui::vec2(crate::ui::larghezza(ui), 30.0)),
            )
            .clicked()
        {
            stato.codice_sblocco = Some(genera_codice());
            stato.testo_sblocco.clear();
            stato.lunghezza_precedente = 0;
        }
        return;
    }

    let codice = stato.codice_sblocco.clone().unwrap_or_default();
    ui.label(
        egui::RichText::new("Digita questo codice a mano per fermarti:")
            .size(13.0)
            .color(GIALLO),
    );
    ui.label(egui::RichText::new(&codice).size(13.0).strong().monospace());
    let risposta = egui::TextEdit::singleline(&mut stato.testo_sblocco)
        .desired_width(f32::INFINITY)
        .show(ui);
    if risposta.response.changed() {
        verifica_incolla(stato, ctx);
    }
    ui.horizontal(|ui| {
        if ui.button("✅ Conferma").clicked() && stato.testo_sblocco == codice {
            stato.codice_sblocco = None;
            stato.testo_sblocco.clear();
            audio.ferma_sottofondo();
            stato.termina_sessione();
            stato.aggiungi_notifica("Sessione interrotta. Ok riprendere quando vuoi.");
        }
        if ui.button("Annulla").clicked() {
            stato.codice_sblocco = None;
            stato.testo_sblocco.clear();
        }
    });
}

/// Rifiuta il copia-incolla del codice.
///
/// Guarda gli eventi di inserimento, non la lunghezza del campo: cosi'
/// selezionare tutto e riscrivere a mano resta un'azione lecita.
fn verifica_incolla(stato: &mut StatoApp, ctx: &egui::Context) {
    let (incolla, inseriti) = ctx.input(|i| {
        let incolla = i.events.iter().any(|e| matches!(e, egui::Event::Paste(_)));
        let inseriti: usize = i
            .events
            .iter()
            .filter_map(|e| match e {
                egui::Event::Text(testo) => Some(testo.chars().count()),
                _ => None,
            })
            .sum();
        (incolla, inseriti)
    });

    if incolla || inseriti > MAX_CARATTERI_PER_FRAME {
        stato.testo_sblocco.clear();
        stato.lunghezza_precedente = 0;
        stato.avviso = Some(
            "Copia-incolla non consentito: digita il codice a mano, carattere per carattere."
                .into(),
        );
        return;
    }
    stato.lunghezza_precedente = stato.testo_sblocco.chars().count();
}

/// Genera un codice casuale da digitare.
fn genera_codice() -> String {
    const ALFABETO: &[u8] = b"abcdefghilmnoprstuvz0123456789";
    let mut rng = StdRng::from_entropy();
    (0..LUNGHEZZA_CODICE)
        .map(|_| ALFABETO[rng.gen_range(0..ALFABETO.len())] as char)
        .collect()
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn il_codice_ha_la_lunghezza_prevista() {
        assert_eq!(genera_codice().chars().count(), LUNGHEZZA_CODICE);
    }

    #[test]
    fn il_codice_e_diverso_ogni_volta() {
        assert_ne!(genera_codice(), genera_codice());
    }
}
