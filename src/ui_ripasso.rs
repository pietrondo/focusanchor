//! Pannello di ripasso: le carte scadute, una alla volta.
//!
//! Lo stesso flusso serve due posti: il riquadro nel setup (dove vedi quante
//! carte hai) e l'overlay durante la sessione (dove il ripasso serve davvero).
//! La carta e i suoi pulsanti stanno in `carta_attuale`, cosi' i due punti
//! d'ingresso non si possono scollare.

use eframe::egui;

use crate::carte::{self, Giudizio, TETTO_GIORNALIERO};
use crate::state::StatoApp;
use crate::ui::{ACCENTO, BORDO, GIALLO, TENUE, VERDE};

/// Riquadro completo: quante carte hai e quante scadono oggi.
pub fn sezione_ripasso(ui: &mut egui::Ui, stato: &mut StatoApp) {
    stato.sincronizza_memoria();
    let oggi = carte::oggi();
    let scadute = stato.memoria.scadute(&oggi);
    let da_oggi = stato.memoria.scadute_limite(&oggi, TETTO_GIORNALIERO);
    let totale = stato.memoria.carte.len();

    if totale == 0 {
        ui.label(
            egui::RichText::new(
                "Nessuna carta. Le carte nascono dalle domande a tradimento: \
                 rispondi a una durante la sessione e la ritrovi qui.",
            )
            .size(13.0)
            .color(TENUE),
        );
        pulsante_genera(ui, stato);
        return;
    }

    ui.horizontal(|ui| {
        crate::ui::badge(ui, &format!("{totale} carte"), ACCENTO);
        let colore = if scadute.is_empty() { VERDE } else { GIALLO };
        crate::ui::badge(ui, &format!("{} da ripassare", scadute.len()), colore);
    });

    if scadute.is_empty() {
        ui.label(
            egui::RichText::new("Niente da ripassare oggi: torna domani.")
                .size(13.0)
                .color(TENUE),
        );
        return;
    }

    // Il tetto: meglio 15 carte fatte bene che 40 abbandonate a meta'.
    if scadute.len() > da_oggi.len() {
        let rimaste = scadute.len() - da_oggi.len();
        ui.label(
            egui::RichText::new(format!(
                "Oggi ne fai {TETTO_GIORNALIERO}: le altre {rimaste} aspettano domani, \
                 non si perdono."
            ))
            .size(12.0)
            .color(TENUE),
        );
    }

    if stato.ripasso_indice.is_none() {
        if ui
            .add(
                egui::Button::new(egui::RichText::new("🔁 Inizia il ripasso").strong())
                    .min_size(egui::vec2(crate::ui::larghezza(ui), 36.0)),
            )
            .clicked()
        {
            stato.ripasso_indice = stato.memoria.prossima_del_giro(&oggi, None);
            stato.ripasso_risposta_visibile = false;
        }
        return;
    }
    carta_attuale(ui, stato, &oggi);
}

/// Versione per l'overlay: compare solo se c'e' qualcosa da ripassare.
///
/// Durante lo studio il ripasso non deve rubare spazio quando non serve, ma
/// deve essere li' nel momento in cui una carta scade.
pub fn ripasso_rapido(ui: &mut egui::Ui, stato: &mut StatoApp, larghezza: f32) {
    stato.sincronizza_memoria();
    let oggi = carte::oggi();
    if stato.ripasso_indice.is_none() && stato.memoria.scadute(&oggi).is_empty() {
        return;
    }
    ui.add_space(6.0);
    if stato.ripasso_indice.is_none() {
        // Il pulsante promette il giro di oggi, non tutto il backlog.
        let quante = stato.memoria.scadute_limite(&oggi, carte::TETTO_GIORNALIERO).len();
        if ui
            .add(
                egui::Button::new(format!("🔁 {quante} carte da ripassare"))
                    .min_size(egui::vec2(larghezza, 30.0)),
            )
            .clicked()
        {
            stato.ripasso_indice = stato.memoria.prossima_del_giro(&oggi, None);
            stato.ripasso_risposta_visibile = false;
        }
        return;
    }
    carta_attuale(ui, stato, &oggi);
}

/// La carta in corso: domanda, risposta e i tre giudizi.
fn carta_attuale(ui: &mut egui::Ui, stato: &mut StatoApp, oggi: &str) {
    let Some(indice) = stato.ripasso_indice else {
        return;
    };
    // Si copia subito: il prestito su `memoria` non deve sopravvivere alla
    // chiusura della carta, perche' dopo si modifica lo stato.
    let Some((domanda, risposta)) = stato
        .memoria
        .carte
        .get(indice)
        .map(|carta| (carta.domanda.clone(), carta.risposta.clone()))
    else {
        stato.ripasso_indice = None;
        return;
    };

    egui::Frame::none()
        .fill(egui::Color32::from_rgb(21, 24, 30))
        .stroke(egui::Stroke::new(1.0, BORDO))
        .rounding(egui::Rounding::same(8.0))
        .inner_margin(egui::Margin::same(10.0))
        .show(ui, |ui| {
            ui.set_min_width(crate::ui::larghezza(ui));
            ui.label(
                egui::RichText::new("DOMANDA")
                    .size(10.0)
                    .strong()
                    .color(TENUE),
            );
            ui.label(egui::RichText::new(crate::ui::accorcia(&domanda, 400)).size(14.0));
            ui.add_space(6.0);
            if stato.ripasso_risposta_visibile {
                ui.label(
                    egui::RichText::new("LA TUA RISPOSTA")
                        .size(10.0)
                        .strong()
                        .color(TENUE),
                );
                egui::ScrollArea::vertical()
                    .max_height(150.0)
                    .show(ui, |ui| {
                        ui.label(egui::RichText::new(&risposta).size(14.0).color(ACCENTO));
                    });
            } else if ui.button("👁 Mostra la risposta").clicked() {
                stato.ripasso_risposta_visibile = true;
            }
        });

    if !stato.ripasso_risposta_visibile {
        return;
    }
    // Senza giudizio la carta non si sposta: e' quello che rende utile il giro.
    ui.label(
        egui::RichText::new("Come e' andata?")
            .size(12.0)
            .color(TENUE),
    );
    let meta = ((crate::ui::larghezza(ui) - 10.0) / 3.0).max(80.0);
    ui.horizontal(|ui| {
        for (etichetta, giudizio) in [
            ("❌ Non lo sapevo", Giudizio::Male),
            ("🤔 Così così", Giudizio::Medio),
            ("✅ Lo sapevo", Giudizio::Bene),
        ] {
            if ui
                .add(egui::Button::new(etichetta).min_size(egui::vec2(meta, 34.0)))
                .clicked()
            {
                giudica(stato, indice, giudizio, oggi);
            }
        }
    });
}

/// Crea carte dalle sottolineature lette da Zotero.
///
/// E' l'unico modo di avere qualcosa da ripassare prima della prima sessione,
/// e usa materiale che hai gia' scelto tu.
fn pulsante_genera(ui: &mut egui::Ui, stato: &mut StatoApp) {
    if stato.evidenziazioni.is_empty() {
        ui.label(
            egui::RichText::new(
                "Per generarle subito, carica un esame con le sue sottolineature da Zotero.",
            )
            .size(12.0)
            .color(TENUE),
        );
        return;
    }
    let quante = stato.evidenziazioni.len();
    if ui
        .add(
            egui::Button::new(format!("📄 Genera dalle {quante} sottolineature"))
                .min_size(egui::vec2(crate::ui::larghezza(ui), 32.0)),
        )
        .on_hover_text("Una carta per ogni passaggio che hai evidenziato in Zotero")
        .clicked()
    {
        let esame = stato.memoria_esame.clone();
        match crate::carte_sottolineature::genera(&esame, &stato.evidenziazioni) {
            Ok((memoria, aggiunte)) => {
                stato.memoria = memoria;
                stato.aggiungi_notifica(&format!("{aggiunte} carte create dalle sottolineature."));
            }
            Err(e) => stato.avviso = Some(e),
        }
    }
}

/// Registra il giudizio, salva e passa alla carta successiva.
fn giudica(stato: &mut StatoApp, indice: usize, giudizio: Giudizio, oggi: &str) {
    let giorno = chrono::Local::now().date_naive();
    if let Some(carta) = stato.memoria.carte.get_mut(indice) {
        carte::ripassa(carta, giudizio, giorno);
    }
    if let Err(e) = carte::salva(&stato.memoria_esame.clone(), &stato.memoria) {
        stato.avviso = Some(e);
    }
    stato.ripasso_risposta_visibile = false;
    // Il giro si ferma al tetto: le carte oltre restano scadute e tornano
    // domani. Meglio un giro finito che uno abbandonato a meta'.
    stato.ripasso_indice = stato.memoria.prossima_del_giro(oggi, Some(indice));
}
