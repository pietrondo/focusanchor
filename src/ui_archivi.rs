//! Sezione dei file dell'utente e export per NotebookLM.

use eframe::egui;

use crate::archivi;
use crate::state::StatoApp;
use crate::ui::{mostra_contenuto, TENUE, VIOLA};

/// Visualizzatori dei file e generazione del pacchetto NotebookLM.
pub fn sezione_archivi(ui: &mut egui::Ui, stato: &mut StatoApp) {
    ui.horizontal(|ui| {
        if ui.button("📝 Pensieri").clicked() {
            mostra_contenuto(ui, "parcheggio_pensieri.txt", &archivi::leggi_parcheggio());
        }
        if ui.button("📄 Riassunti").clicked() {
            match archivi::leggi_riassunti() {
                Ok(testo) => mostra_contenuto(ui, "riassunto_studio.md", &testo),
                Err(e) => segnala_errore_materiali(stato, &e),
            }
        }
    });
    ui.horizontal(|ui| {
        if ui.button("🗑️ Svuota pensieri").clicked() {
            match archivi::svuota_parcheggio() {
                Ok(()) => stato.aggiungi_notifica("Parcheggio pensieri svuotato."),
                Err(e) => stato.avviso = Some(e),
            }
        }
        if ui.button("📓 Scheda NotebookLM").clicked() {
            let esame = stato.nome_esame();
            match archivi::leggi_notebooklm_esame_result(&esame) {
                Ok(testo) if !testo.trim().is_empty() => mostra_contenuto(
                    ui,
                    &archivi::nome_file_notebooklm(&esame),
                    &testo,
                ),
                Ok(_) => segnala_errore_materiali(stato, "La scheda NotebookLM è vuota."),
                Err(e) => segnala_errore_materiali(stato, &e),
            }
        }
    });
    ui.add_space(6.0);

    sezione_anki(ui, stato);

    ui.add_space(6.0);

    if ui
        .add(
            egui::Button::new(
                egui::RichText::new("🚀 Genera scheda per NotebookLM")
                    .size(15.0)
                    .strong(),
            )
            .fill(VIOLA.gamma_multiply(0.35))
            .min_size(egui::vec2(crate::ui::larghezza(ui), 40.0)),
        )
        .clicked()
    {
        genera_notebooklm(stato);
    }
    ui.label(
        egui::RichText::new(
            "Crea un file per esame con tutte le sottolineature e i tuoi riassunti, \
             pronto da caricare su NotebookLM per l'Audio Overview.",
        )
        .size(12.0)
        .color(TENUE),
    );

    // Il caricamento su NotebookLM si fa a mano: aprire il file nel file
    // system evita di doverlo cercare.
    let percorso = archivi::percorso_notebooklm_esame(&stato.nome_esame());
    if percorso.exists() && ui.small_button("📂 Mostra il file da caricare").clicked() {
        if let Err(e) = archivi::rivela_file(&percorso) {
            stato.avviso = Some(e);
        }
    }
}

/// Pulsante e spiegazione delle carte per Anki.
fn sezione_anki(ui: &mut egui::Ui, stato: &mut StatoApp) {
    if ui
        .add(
            egui::Button::new(
                egui::RichText::new("📇 Carte per Anki").size(15.0).strong(),
            )
            .min_size(egui::vec2(crate::ui::larghezza(ui), 36.0)),
        )
        .on_hover_text("Le tue risposte, pronte per il ripasso dilazionato")
        .clicked()
    {
        let esame = stato.nome_esame();
        mostra_contenuto(
            ui,
            &format!("anki_{}.txt", archivi::nome_file_sicuro(&esame)),
            &archivi::leggi_anki(&esame),
        );
    }
    ui.label(
        egui::RichText::new(
            "Ogni risposta alle domande a tradimento diventa una carta. \
             Importa il file in Anki (formato tabulato) per ripassare nei giorni giusti.",
        )
        .size(12.0)
        .color(TENUE),
    );

    let percorso = archivi::percorso_anki(&stato.nome_esame());
    if percorso.exists() && ui.small_button("📂 Mostra il file per Anki").clicked() {
        if let Err(e) = archivi::rivela_file(&percorso) {
            stato.avviso = Some(e);
        }
    }
}

/// Genera il pacchetto NotebookLM dell'esame corrente.
///
/// Le sottolineature vengono rilette da Zotero al momento, cosi' il file
/// contiene anche quello che l'utente ha evidenziato durante la sessione.
pub(crate) fn genera_notebooklm(stato: &mut StatoApp) {
    let aggiornamento = aggiorna_materiali_notebooklm(stato);
    genera_dopo_aggiornamento(stato, aggiornamento);
}

fn aggiorna_materiali_notebooklm(stato: &mut StatoApp) -> Result<(), String> {
    let Some(id) = stato.esame_scelto else {
        crate::ui_esami::invalida_materiali_zotero(stato);
        return Err("Seleziona un esame prima di generare la scheda.".into());
    };
    let cartella = crate::config::cartella_dati();
    crate::ui_esami::aggiorna_materiali_zotero(stato, id, &cartella, 400)
}

fn genera_dopo_aggiornamento(stato: &mut StatoApp, aggiornamento: Result<(), String>) {
    if let Err(e) = aggiornamento {
        segnala_errore_materiali(stato, &format!("Scheda non aggiornata: {e}"));
        return;
    }
    let esame = stato.nome_esame();
    let riassunti = match archivi::leggi_riassunti() {
        Ok(riassunti) => riassunti,
        Err(e) => {
            segnala_errore_materiali(stato, &e);
            return;
        }
    };
    let risultato = archivi::genera_pacchetto_notebooklm(&esame, &stato.evidenziazioni, &riassunti);
    finalizza_generazione(stato, &esame, risultato);
}

fn finalizza_generazione(stato: &mut StatoApp, esame: &str, risultato: Result<(), String>) {
    match risultato {
        Ok(()) => {
            stato.anteprima_notebooklm = None;
            segnala_successo_materiali(stato, &format!(
                "Scheda pronta: {}",
                archivi::nome_file_notebooklm(esame)
            ));
        }
        Err(e) => segnala_errore_materiali(stato, &e),
    }
}

pub(crate) fn segnala_errore_materiali(stato: &mut StatoApp, messaggio: &str) {
    stato.anteprima_notebooklm = None;
    stato.conferma = None;
    stato.avviso = Some(messaggio.to_string());
}

pub(crate) fn segnala_successo_materiali(stato: &mut StatoApp, messaggio: &str) {
    stato.avviso = None;
    stato.aggiungi_notifica(messaggio);
}

#[cfg(test)]
mod test_aggiornamento_notebooklm {
    use super::*;
    use crate::zotero::Evidenziazione;

    #[test]
    fn il_refresh_fallito_non_riusa_annotazioni_stale() {
        let mut stato = StatoApp::nuovo();
        stato.esame_scelto = Some(17);
        stato.anteprima_notebooklm = Some(("obsolete.md".into(), "old material".into()));
        stato.snapshot_zotero = Some(std::path::PathBuf::from("old_snapshot.sqlite"));
        stato.collezioni = vec![crate::zotero::Collezione {
            id: 17,
            nome: "Esame obsoleto".into(),
            documenti: 1,
            sottocollezioni: 0,
        }];
        stato.documenti_esame = vec![crate::zotero::Documento {
            chiave: "OLD".into(),
            chiave_pdf: None,
            titolo: "Documento obsoleto".into(),
            tipo: "book".into(),
            data_aggiunta: String::new(),
        }];
        stato.testi_scelti = vec!["Documento obsoleto".into()];
        stato.config.cartella_zotero = std::env::temp_dir()
            .join(format!("zotero-missing-{}", std::process::id()))
            .to_string_lossy()
            .into_owned();
        stato.evidenziazioni = vec![Evidenziazione {
            testo: "Annotazione obsoleta".into(),
            commento: String::new(),
            data: "2026-01-01".into(),
        }];

        let risultato = aggiorna_materiali_notebooklm(&mut stato);

        assert!(risultato.is_err());
        assert!(stato.evidenziazioni.is_empty());
        assert!(stato.snapshot_zotero.is_none());
        assert!(stato.collezioni.is_empty());
        assert!(stato.documenti_esame.is_empty());
        assert!(stato.testi_scelti.is_empty());
        assert!(stato.anteprima_notebooklm.is_none());
    }

    #[test]
    fn il_refresh_fallito_interrompe_la_generazione_senza_notifica_di_successo() {
        let mut stato = StatoApp::nuovo();
        stato.conferma = Some(("Scheda pronta precedente".into(), std::time::Instant::now()));
        genera_dopo_aggiornamento(&mut stato, Err("Zotero non disponibile".into()));

        assert!(stato.conferma.is_none());
        assert!(stato
            .avviso
            .as_deref()
            .is_some_and(|avviso| avviso.contains("Zotero non disponibile")));
    }

    #[test]
    fn una_notifica_di_successo_pulisce_l_avviso_precedente() {
        let mut stato = StatoApp::nuovo();
        stato.avviso = Some("Errore precedente".into());

        segnala_successo_materiali(&mut stato, "Testi ricaricati da Zotero.");

        assert!(stato.avviso.is_none());
        assert!(stato
            .conferma
            .as_ref()
            .is_some_and(|(testo, _)| testo == "Testi ricaricati da Zotero."));
    }

    #[test]
    fn la_generazione_riuscita_chiude_l_anteprima_obsoleta() {
        let mut stato = StatoApp::nuovo();
        stato.anteprima_notebooklm = Some(("vecchia.md".into(), "vecchio contenuto".into()));
        stato.avviso = Some("Errore precedente".into());

        finalizza_generazione(&mut stato, "Storia romana", Ok(()));

        assert!(stato.anteprima_notebooklm.is_none());
        assert!(stato.avviso.is_none());
        assert!(stato
            .conferma
            .as_ref()
            .is_some_and(|(testo, _)| testo.contains("Scheda pronta")));
    }

    #[test]
    fn errore_di_scrittura_chiude_anteprima_e_toast_di_successo() {
        let mut stato = StatoApp::nuovo();
        stato.anteprima_notebooklm = Some(("vecchia.md".into(), "vecchio testo".into()));
        stato.conferma = Some(("Scheda pronta precedente".into(), std::time::Instant::now()));

        finalizza_generazione(&mut stato, "Storia romana", Err("disco pieno".into()));

        assert!(stato.anteprima_notebooklm.is_none());
        assert!(stato.conferma.is_none());
        assert!(stato.avviso.as_deref().is_some_and(|a| a.contains("disco pieno")));
    }

    #[test]
    fn errore_di_lettura_chiude_anteprima_e_toast_di_successo() {
        let mut stato = StatoApp::nuovo();
        stato.anteprima_notebooklm = Some(("vecchia.md".into(), "vecchio testo".into()));
        stato.conferma = Some(("Scheda pronta precedente".into(), std::time::Instant::now()));

        segnala_errore_materiali(&mut stato, "file non leggibile");

        assert!(stato.anteprima_notebooklm.is_none());
        assert!(stato.conferma.is_none());
        assert!(stato.avviso.as_deref().is_some_and(|a| a.contains("file non leggibile")));
    }
}
