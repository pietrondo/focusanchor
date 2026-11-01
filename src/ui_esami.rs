//! Logica di caricamento da Zotero e avvio della sessione.
//!
//! Separata dalla grafica: qui si legge il database, si sceglie l'esame e si
//! fa partire la sessione con i testi spuntati.

use std::sync::mpsc::Sender;

use crate::audio::MotoreAudio;
use crate::blocker::Segnale;
use crate::interleaving::Interleaving;
use crate::state::StatoApp;
use crate::zotero_query;

/// Cambia esame e ricarica i testi della nuova collezione.
pub fn cambia_esame(stato: &mut StatoApp, nuovo: Option<i64>) {
    stato.esame_scelto = nuovo;
    stato.documenti_esame.clear();
    stato.testi_scelti.clear();
    stato.evidenziazioni.clear();

    let (Some(id), Some(snapshot)) = (nuovo, stato.snapshot_zotero.clone()) else {
        return;
    };
    match zotero_query::documenti_collezione(&snapshot, id, 60) {
        Ok(documenti) => stato.documenti_esame = documenti,
        Err(e) => stato.avviso = Some(format!("Documenti dell'esame: {e}")),
    }
    match zotero_query::evidenziazioni_esame(&snapshot, Some(id), 40) {
        Ok(eviden) => stato.evidenziazioni = eviden,
        Err(e) => stato.avviso = Some(format!("Sottolineature dell'esame: {e}")),
    }
}

/// Ricarica i testi dell'esame tenendo le scelte ancora valide.
///
/// Serve quando l'utente aggiunge libri in Zotero durante la preparazione:
/// non deve perdere i testi gia' spuntati.
pub fn ricarica_testi(stato: &mut StatoApp) {
    let (Some(id), Some(snapshot)) = (stato.esame_scelto, stato.snapshot_zotero.clone()) else {
        return;
    };
    match zotero_query::documenti_collezione(&snapshot, id, 60) {
        Ok(documenti) => {
            stato.documenti_esame = documenti;
            // Tengo solo le scelte che esistono ancora.
            stato
                .testi_scelti
                .retain(|t| stato.documenti_esame.iter().any(|d| &d.titolo == t));
            stato.aggiungi_notifica("Testi ricaricati da Zotero.");
            if let Ok(eviden) = zotero_query::evidenziazioni_esame(&snapshot, Some(id), 40) {
                stato.evidenziazioni = eviden;
            }
        }
        Err(e) => stato.avviso = Some(format!("Ricarica testi: {e}")),
    }
}

/// Carica collezioni e documenti dal database di Zotero.
pub fn carica_esami(stato: &mut StatoApp) {
    let cartella = crate::config::cartella_dati();
    let percorso = stato.config.cartella_zotero.clone();
    let snapshot = match crate::zotero::crea_snapshot_con(&cartella, &percorso) {
        Ok(s) => s,
        Err(e) => {
            stato.avviso = Some(e);
            return;
        }
    };
    match zotero_query::collezioni(&snapshot) {
        Ok(elenco) => {
            stato.collezioni = elenco;
            stato.snapshot_zotero = Some(snapshot);
            stato.avviso = None;
        }
        Err(e) => {
            stato.avviso = Some(format!("Lettura collezioni fallita: {e}"));
            return;
        }
    }
    // Se c'e' un solo esame conviene selezionarlo subito.
    if stato.collezioni.len() == 1 {
        let id = stato.collezioni[0].id;
        cambia_esame(stato, Some(id));
    }
}

/// Rileva il PDF aperto su Zotero e lo usa come obiettivo.
pub fn rileva_pdf_zotero(stato: &mut StatoApp) {
    match crate::finestre::pdf_aperto_su_zotero() {
        Some(titolo) => {
            stato.config.obiettivo = titolo.clone();
            stato.aggiungi_notifica(&format!("Zotero: {titolo}"));
        }
        None => stato.avviso = Some(
            "Non trovo una finestra di Zotero con un PDF aperto. Apri il PDF su Zotero.".into(),
        ),
    }
}

/// Avvia la sessione di studio con i testi spuntati.
///
/// Non tocca il file `hosts`: il blocco dei siti e' stato rimosso, quindi non
/// serve piu' l'avvio come Amministratore.
pub fn avvia_sessione(
    stato: &mut StatoApp,
    audio: &mut MotoreAudio,
    mittente: &Sender<Segnale>,
) {
    stato.salva_configurazione();

    // I testi spuntati diventano la rotazione della sessione.
    stato.interleaving = Interleaving::nuovo(stato.testi_scelti.clone());
    if stato.testi_scelti.len() >= 2 {
        stato.config.obiettivo = stato.testi_scelti[0].clone();
    }

    let _ = audio.imposta_modalita(stato.modalita_audio(), stato.config.volume);
    stato.avvia_sessione(mittente);
    let obiettivo = stato.obiettivo_corrente();
    audio.parla(&format!("Sessione avviata. Studiamo {obiettivo}. Iniziamo."));
}
