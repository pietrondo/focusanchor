//! Sezione dei file dell'utente e export per NotebookLM.

use eframe::egui;

use crate::archivi;
use crate::state::StatoApp;
use crate::ui::{mostra_contenuto, TENUE, VIOLA};
use crate::zotero_query;

/// Visualizzatori dei file e generazione del pacchetto NotebookLM.
pub fn sezione_archivi(ui: &mut egui::Ui, stato: &mut StatoApp) {
    ui.horizontal(|ui| {
        if ui.button("📝 Pensieri").clicked() {
            mostra_contenuto(ui, "parcheggio_pensieri.txt", &archivi::leggi_parcheggio());
        }
        if ui.button("📄 Riassunti").clicked() {
            mostra_contenuto(ui, "riassunto_studio.md", &archivi::leggi_riassunti());
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
            mostra_contenuto(
                ui,
                &archivi::nome_file_notebooklm(&esame),
                &archivi::leggi_notebooklm_esame(&esame),
            );
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
fn genera_notebooklm(stato: &mut StatoApp) {
    let esame = stato.nome_esame();

    // Ricarica le sottolineature dell'esame: possono essere cresciute.
    if let (Some(snapshot), Some(id)) = (stato.snapshot_zotero.clone(), stato.esame_scelto) {
        if let Ok(eviden) = zotero_query::evidenziazioni_esame(&snapshot, Some(id), 400) {
            stato.evidenziazioni = eviden;
        }
    }

    let riassunti = archivi::leggi_riassunti();
    match archivi::genera_pacchetto_notebooklm(&esame, &stato.evidenziazioni, &riassunti) {
        Ok(()) => stato.aggiungi_notifica(&format!(
            "Scheda pronta: {}",
            archivi::nome_file_notebooklm(&esame)
        )),
        Err(e) => stato.avviso = Some(e),
    }
}
