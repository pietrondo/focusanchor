//! Riquadro degli appelli: le date di tutti gli esami in una vista sola.
//!
//! Il conto alla rovescia durante la sessione mostra solo l'esame piu'
//! vicino. Qui si vedono tutti: con piu' appelli a dicembre serve sapere
//! quale viene prima, non solo quando scade il primo.
//!
//! Le date si scrivono come sul portale dell'ateneo (`GG/MM/AAAA`): e' il
//! formato in cui le hai gia' davanti agli occhi.

use eframe::egui;

use crate::appelli::{self, Appello, Piano};
use crate::state::StatoApp;
use crate::ui::{badge, ACCENTO, ARANCIO, GIALLO, TENUE, VERDE};

/// Riquadro "Appelli": elenco degli esami con data, conto alla rovescia e piano.
pub fn sezione_appelli(ui: &mut egui::Ui, stato: &mut StatoApp) {
    let elenco = appelli::elenco(&stato.config, &stato.storico);
    if elenco.is_empty() {
        ui.label(
            egui::RichText::new(
                "Nessun appello in lista. Apri un esame da Zotero e salva la sua scheda: \
                 qui compariranno le date e il conto alla rovescia.",
            )
            .size(13.0)
            .color(TENUE),
        );
        return;
    }

    let secondi = appelli::secondi_per_documento(&stato.storico);
    for appello in &elenco {
        let piano = appelli::piano(&appello.nome, &secondi, &stato.collezioni, appello.giorni);
        let ritmo = appelli::ritmo_esame(&stato.storico, &appello.nome);
        riga_appello(ui, stato, appello, &piano, ritmo);
    }
}

/// Una riga per esame: nome, data modificabile, quanto manca, il piano.
fn riga_appello(
    ui: &mut egui::Ui,
    stato: &mut StatoApp,
    appello: &Appello,
    piano: &Piano,
    ritmo: u32,
) {
    ui.horizontal_wrapped(|ui| {
        // Il nome e' cliccabile: scegliere l'esame e' il ponte fra "quando
        // ho esami" e "cosa studio adesso".
        let nome = crate::ui::accorcia(&appello.nome, 26);
        let gia_scelto = stato.esame_scelto
            .and_then(|id| stato.collezioni.iter().find(|c| c.id == id))
            .map(|c| c.nome == appello.nome)
            .unwrap_or(false);
        let risposta = ui.selectable_label(
            gia_scelto,
            egui::RichText::new(nome).size(15.0).strong(),
        );
        if risposta.clicked() && !gia_scelto {
            scegli_esame(stato, &appello.nome);
        }
        campo_data(ui, stato, &appello.nome);
        match appello.giorni {
            Some(giorni) if giorni < 0 => badge(ui, "passato", TENUE),
            Some(0) => badge(ui, "oggi", GIALLO),
            Some(giorni) if giorni <= 7 => badge(ui, &format!("fra {giorni} giorni"), ARANCIO),
            Some(giorni) => badge(ui, &format!("fra {giorni} giorni"), ACCENTO),
            None => badge(ui, "senza data", TENUE),
        }
        if appello.minuti > 0 {
            badge(ui, &format!("{} min", appello.minuti), VERDE);
        }
        if appello.crediti > 0 {
            badge(ui, &format!("{} CF", appello.crediti), TENUE);
        }
    });

    // Il piano di studio: quanto resta e come dividerlo sui giorni che
    // mancano. E' la differenza fra "40 giorni" e "40 occasioni da 25
    // minuti".
    let mut dettagli: Vec<String> = Vec::new();
    if let (Some(totale), letti) = (piano.documenti, piano.letti) {
        dettagli.push(format!("{letti}/{totale} documenti"));
    }
    if let Some(minuti) = piano.minuti_al_giorno {
        dettagli.push(format!("~{minuti} min/giorno per arrivarci"));
    } else if let Some(minuti) = piano.minuti_rimanenti {
        dettagli.push(format!("~{minuti} min di materiale da finire"));
    }
    if let (Some(giorni), true) = (appello.giorni, ritmo > 0) {
        if giorni > 0 {
            let ore = (ritmo as i64 * giorni) / 60;
            dettagli.push(format!("al tuo ritmo hai ~{ore} h"));
        }
    }
    if !dettagli.is_empty() {
        ui.label(
            egui::RichText::new(dettagli.join(" · "))
                .size(12.0)
                .color(TENUE),
        );
    }
}

/// Seleziona un esame dall'elenco appelli e ne carica i testi.
///
/// Il percorso e' quello del menu esami: se Zotero non e' caricato non si
/// puo' cambiare esame, e l'app lo dice.
fn scegli_esame(stato: &mut StatoApp, nome: &str) {
    let Some(id) = stato
        .collezioni
        .iter()
        .find(|c| c.nome == nome)
        .map(|c| c.id)
    else {
        return;
    };
    crate::ui_esami::cambia_esame(stato, Some(id));
}

/// Campo della data d'appello: accetta `GG/MM/AAAA` e `AAAA-MM-GG`.
///
/// Il testo digitato vive nello stato: un campo ricreato a ogni frame
/// ripartirebbe dal valore salvato e perderebbe i caratteri.
pub fn campo_data(ui: &mut egui::Ui, stato: &mut StatoApp, esame: &str) {
    let salvata = stato
        .config
        .esami
        .get(esame)
        .map(|info| info.data_esame.clone())
        .unwrap_or_default();
    let mut testo = stato
        .date_in_edizione
        .get(esame)
        .cloned()
        .unwrap_or(salvata);
    let risposta = egui::TextEdit::singleline(&mut testo)
        // Il suggerimento dice il formato, non una data: un placeholder
        // con una data vera sembra un appello gia' impostato.
        .hint_text("GG/MM/AAAA")
        .desired_width(105.0)
        .show(ui);
    let cambiato = risposta.response.changed();
    stato.date_in_edizione.insert(esame.to_string(), testo.clone());
    if !cambiato {
        return;
    }

    let pulito = testo.trim().to_string();
    if pulito.is_empty() {
        stato
            .config
            .esami
            .entry(esame.to_string())
            .or_default()
            .data_esame
            .clear();
        stato.salva_configurazione();
        return;
    }
    // Una data illeggibile non viene salvata: meglio nessuna data che una
    // data sbagliata che sembra vera.
    if crate::storico::parse_data(&pulito).is_none() {
        return;
    }
    stato
        .config
        .esami
        .entry(esame.to_string())
        .or_default()
        .data_esame = pulito;
    stato.salva_configurazione();
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::config::InfoEsame;

    /// Il campo salva la data solo se e' leggibile: e' la logica che il
    /// widget egui chiama a ogni frame.
    fn salva_data(stato: &mut StatoApp, esame: &str, testo: &str) {
        let pulito = testo.trim().to_string();
        if pulito.is_empty() {
            stato
                .config
                .esami
                .entry(esame.to_string())
                .or_default()
                .data_esame
                .clear();
            stato.salva_configurazione();
            return;
        }
        if crate::storico::parse_data(&pulito).is_none() {
            return;
        }
        stato
            .config
            .esami
            .entry(esame.to_string())
            .or_default()
            .data_esame = pulito;
        stato.salva_configurazione();
    }

    #[test]
    fn la_data_salva_sia_in_formato_italiano_che_iso() {
        let mut stato = StatoApp::nuovo();
        stato.config.esami.clear();

        salva_data(&mut stato, "storia romana", "09/12/2026");
        assert_eq!(
            stato.config.esami["storia romana"].data_esame,
            "09/12/2026"
        );

        salva_data(&mut stato, "storia romana", "2026-12-16");
        assert_eq!(
            stato.config.esami["storia romana"].data_esame,
            "2026-12-16"
        );
    }

    #[test]
    fn una_data_illeggibile_non_viene_salvata() {
        let mut stato = StatoApp::nuovo();
        stato.config.esami.clear();
        stato.config.esami.insert(
            "storia romana".into(),
            InfoEsame {
                data_esame: "2026-12-16".into(),
                ..Default::default()
            },
        );

        salva_data(&mut stato, "storia romana", "domani");

        // La data precedente resta: una data illeggibile non cancella
        // l'appello che c'era.
        assert_eq!(
            stato.config.esami["storia romana"].data_esame,
            "2026-12-16"
        );
    }
}
