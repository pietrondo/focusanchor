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


/// Invalida i dati derivati da Zotero quando un refresh non e' completo.
pub(crate) fn invalida_materiali_zotero(stato: &mut StatoApp) {
    stato.snapshot_zotero = None;
    stato.collezioni.clear();
    stato.esame_scelto = None;
    stato.documenti_esame.clear();
    stato.testi_scelti.clear();
    stato.evidenziazioni.clear();
    stato.anteprima_notebooklm = None;
    stato.campo_notebook.clear();
}

/// Carica entrambi gli insiemi dal nuovo snapshot prima di pubblicarli nello stato.
fn applica_snapshot_materiali(
    stato: &mut StatoApp,
    id: i64,
    snapshot: &std::path::Path,
) -> Result<(), String> {
    match leggi_materiali(snapshot, id, 60, 40) {
        Ok((documenti, evidenziazioni)) => {
            stato.snapshot_zotero = Some(snapshot.to_path_buf());
            stato.esame_scelto = Some(id);
            stato.documenti_esame = documenti;
            stato.evidenziazioni = evidenziazioni;
            stato.anteprima_notebooklm = None;
            stato
                .testi_scelti
                .retain(|titolo| stato.documenti_esame.iter().any(|d| &d.titolo == titolo));
            Ok(())
        }
        Err(e) => {
            invalida_materiali_zotero(stato);
            Err(e)
        }
    }
}

fn leggi_materiali(
    snapshot: &std::path::Path,
    id: i64,
    limite_documenti: usize,
    limite_annotazioni: usize,
) -> Result<(Vec<crate::zotero::Documento>, Vec<crate::zotero::Evidenziazione>), String> {
    let documenti = zotero_query::documenti_collezione(snapshot, id, limite_documenti)
        .map_err(|e| format!("Lettura documenti Zotero fallita: {e}"))?;
    let evidenziazioni = zotero_query::evidenziazioni_esame(snapshot, Some(id), limite_annotazioni)
        .map_err(|e| format!("Lettura annotazioni Zotero fallita: {e}"))?;
    Ok((documenti, evidenziazioni))
}

/// Aggiorna snapshot e materiali selezionati come un'unica operazione.
pub(crate) fn aggiorna_materiali_zotero(
    stato: &mut StatoApp,
    id: i64,
    cartella_app: &std::path::Path,
    limite_annotazioni: usize,
) -> Result<(), String> {
    let snapshot = match crate::zotero::crea_snapshot_con(
        cartella_app,
        &stato.config.cartella_zotero,
    ) {
        Ok(snapshot) => snapshot,
        Err(e) => {
            invalida_materiali_zotero(stato);
            return Err(e);
        }
    };
    let collezioni = match zotero_query::collezioni(&snapshot) {
        Ok(collezioni) => collezioni,
        Err(e) => {
            invalida_materiali_zotero(stato);
            return Err(format!("Lettura collezioni Zotero fallita: {e}"));
        }
    };
    if !collezioni.iter().any(|collezione| collezione.id == id) {
        invalida_materiali_zotero(stato);
        return Err("L'esame selezionato non è più presente in Zotero.".into());
    }
    match leggi_materiali(&snapshot, id, 60, limite_annotazioni) {
        Ok((documenti, evidenziazioni)) => {
            let esame = collezioni
                .iter()
                .find(|collezione| collezione.id == id)
                .map(|collezione| collezione.nome.as_str());
            let campo_notebook = esame
                .and_then(|nome| stato.config.notebook_esami.get(nome).cloned())
                .unwrap_or_default();
            stato.snapshot_zotero = Some(snapshot);
            stato.collezioni = collezioni;
            stato.esame_scelto = Some(id);
            stato.documenti_esame = documenti;
            stato.evidenziazioni = evidenziazioni;
            stato.anteprima_notebooklm = None;
            stato.campo_notebook = campo_notebook;
            stato
                .testi_scelti
                .retain(|titolo| stato.documenti_esame.iter().any(|d| &d.titolo == titolo));
            Ok(())
        }
        Err(e) => {
            invalida_materiali_zotero(stato);
            Err(e)
        }
    }
}

/// Cambia esame e ricarica i testi della nuova collezione.
pub fn cambia_esame(stato: &mut StatoApp, nuovo: Option<i64>) {
    let Some(id) = nuovo else {
        stato.conferma = None;
        stato.esame_scelto = None;
        stato.documenti_esame.clear();
        stato.testi_scelti.clear();
        stato.evidenziazioni.clear();
        stato.anteprima_notebooklm = None;
        stato.campo_notebook.clear();
        return;
    };
    let esame = stato.collezioni.iter().find(|c| c.id == id).map(|c| c.nome.clone());
    let Some(esame) = esame else {
        invalida_materiali_zotero(stato);
        stato.conferma = None;
        stato.avviso = Some("L'esame selezionato non è più presente in Zotero.".into());
        return;
    };
    let Some(snapshot) = stato.snapshot_zotero.clone() else {
        invalida_materiali_zotero(stato);
        stato.conferma = None;
        stato.avviso = Some("Ricarica Zotero prima di selezionare un esame.".into());
        return;
    };
    match applica_snapshot_materiali(stato, id, &snapshot) {
        Ok(()) => {
            stato.conferma = None;
            stato.campo_notebook = stato.config.notebook_esami.get(&esame).cloned().unwrap_or_default();
            stato.avviso = None;
        }
        Err(e) => {
            invalida_materiali_zotero(stato);
            stato.conferma = None;
            stato.avviso = Some(format!("Materiali dell'esame non aggiornati: {e}"));
        }
    }
}

/// Ricarica i testi dell'esame tenendo le scelte ancora valide.
///
/// Serve quando l'utente aggiunge libri in Zotero durante la preparazione:
/// non deve perdere i testi gia' spuntati. Lo snapshot viene rifatto da zero:
/// rileggere la copia vecchia mostrerebbe sempre la stessa lista, e il
/// pulsante direbbe una bugia.
pub fn ricarica_testi(stato: &mut StatoApp) {
    let Some(id) = stato.esame_scelto else {
        return;
    };
    let cartella = crate::config::cartella_dati();
    match aggiorna_materiali_zotero(stato, id, &cartella, 40) {
        Ok(()) => {
            crate::ui_archivi::segnala_successo_materiali(stato, "Testi ricaricati da Zotero.");
        }
        Err(e) => {
            stato.conferma = None;
            stato.avviso = Some(format!("Ricarica testi: {e}"));
        }
    }
}

/// Carica collezioni e documenti dal database di Zotero.
pub fn carica_esami(stato: &mut StatoApp) {
    let cartella = crate::config::cartella_dati();
    carica_esami_con(stato, &cartella);
}

fn carica_esami_con(stato: &mut StatoApp, cartella_app: &std::path::Path) {
    let percorso = stato.config.cartella_zotero.clone();
    let snapshot = match crate::zotero::crea_snapshot_con(cartella_app, &percorso) {
        Ok(s) => s,
        Err(e) => {
            invalida_materiali_zotero(stato);
            stato.conferma = None;
            stato.avviso = Some(e);
            return;
        }
    };
    let elenco = match zotero_query::collezioni(&snapshot) {
        Ok(elenco) => elenco,
        Err(e) => {
            invalida_materiali_zotero(stato);
            stato.conferma = None;
            stato.avviso = Some(format!("Lettura collezioni fallita: {e}"));
            return;
        }
    };
    if elenco.is_empty() {
        invalida_materiali_zotero(stato);
        stato.conferma = None;
        stato.avviso = Some("Nessun esame trovato nelle collezioni Zotero.".into());
        return;
    }

    let id_precedente = stato.esame_scelto;
    let id = if elenco.len() == 1 {
        Some(elenco[0].id)
    } else {
        id_precedente.filter(|id| elenco.iter().any(|c| c.id == *id))
    };
    let materiali = match id {
        Some(id) => leggi_materiali(&snapshot, id, 60, 40).map(Some),
        None => Ok(None),
    };
    let (documenti, evidenziazioni) = match materiali {
        Ok(Some((documenti, evidenziazioni))) => (documenti, evidenziazioni),
        Ok(None) => (Vec::new(), Vec::new()),
        Err(e) => {
            invalida_materiali_zotero(stato);
            stato.conferma = None;
            stato.avviso = Some(format!("Materiali Zotero non coerenti: {e}"));
            return;
        }
    };
    let campo_notebook = id
        .and_then(|id| elenco.iter().find(|c| c.id == id))
        .and_then(|c| stato.config.notebook_esami.get(&c.nome).cloned())
        .unwrap_or_default();
    stato.snapshot_zotero = Some(snapshot);
    stato.collezioni = elenco;
    stato.esame_scelto = id;
    stato.documenti_esame = documenti;
    stato.evidenziazioni = evidenziazioni;
    stato.campo_notebook = campo_notebook;
    stato.anteprima_notebooklm = None;
    stato
        .testi_scelti
        .retain(|titolo| stato.documenti_esame.iter().any(|d| &d.titolo == titolo));
    stato.avviso = None;
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

    if let Err(messaggio) = audio.imposta_modalita(stato.modalita_audio(), stato.config.volume) {
        stato.avviso = Some(messaggio);
    }
    stato.avvia_sessione(mittente);
    let obiettivo = stato.obiettivo_corrente();
    audio.parla(&format!("Sessione avviata. Studiamo {obiettivo}. Iniziamo."));
}

#[cfg(test)]
mod test_ricarica_atomica {
    use super::*;
    use crate::zotero::{Collezione, Documento, Evidenziazione};
    use rusqlite::Connection;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn snapshot_senza_tabella_annotazioni() -> std::path::PathBuf {
        let nome = format!(
            "focusanchor-zotero-atomic-{}-{}.sqlite",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        );
        let percorso = std::env::temp_dir().join(nome);
        assert!(!percorso.exists());
        let db = Connection::open(&percorso).unwrap();
        db.execute_batch(
            "CREATE TABLE collections(collectionID INTEGER, parentCollectionID INTEGER);
             CREATE TABLE collectionItems(collectionID INTEGER, itemID INTEGER);
             CREATE TABLE items(itemID INTEGER, key TEXT, itemTypeID INTEGER, dateAdded TEXT);
             CREATE TABLE itemAttachments(itemID INTEGER, parentItemID INTEGER, contentType TEXT);
             CREATE TABLE itemTypes(itemTypeID INTEGER, typeName TEXT);
             CREATE TABLE itemData(itemID INTEGER, fieldID INTEGER, valueID INTEGER);
             CREATE TABLE fields(fieldID INTEGER, fieldName TEXT);
             CREATE TABLE itemDataValues(valueID INTEGER, value TEXT);
             CREATE TABLE deletedItems(itemID INTEGER);
             INSERT INTO collections VALUES(17, NULL);
             INSERT INTO collectionItems VALUES(17, 100);
             INSERT INTO items VALUES(100, 'NEW', 1, '2026-11-04 10:00:00');
             INSERT INTO itemTypes VALUES(1, 'book');
             INSERT INTO itemData VALUES(100, 1, 1);
             INSERT INTO fields VALUES(1, 'title');
             INSERT INTO itemDataValues VALUES(1, 'Documento nuovo');",
        )
        .unwrap();
        drop(db);
        percorso
    }

    fn snapshot_vuoto() -> std::path::PathBuf {
        let nome = format!(
            "focusanchor-zotero-empty-{}-{}.sqlite",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        );
        let percorso = std::env::temp_dir().join(nome);
        assert!(!percorso.exists());
        let db = Connection::open(&percorso).unwrap();
        db.execute_batch(
            "CREATE TABLE collections(collectionID INTEGER, collectionName TEXT, parentCollectionID INTEGER);
             CREATE TABLE collectionItems(collectionID INTEGER, itemID INTEGER);
             CREATE TABLE items(itemID INTEGER);
             CREATE TABLE itemAttachments(itemID INTEGER, parentItemID INTEGER);
             CREATE TABLE deletedItems(itemID INTEGER);
             CREATE TABLE deletedCollections(collectionID INTEGER);",
        )
        .unwrap();
        drop(db);
        percorso
    }

    fn snapshot_senza_schema() -> std::path::PathBuf {
        let nome = format!(
            "focusanchor-zotero-invalid-{}-{}.sqlite",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        );
        let percorso = std::env::temp_dir().join(nome);
        assert!(!percorso.exists());
        drop(Connection::open(&percorso).unwrap());
        percorso
    }

    fn cartella_app_test() -> std::path::PathBuf {
        let percorso = std::env::temp_dir().join(format!(
            "focusanchor-ui-esami-{}-{}",
            std::process::id(),
            SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos()
        ));
        assert!(!percorso.exists());
        std::fs::create_dir(&percorso).unwrap();
        percorso
    }

    fn snapshot_con_annotazioni() -> std::path::PathBuf {
        let nome = format!(
            "focusanchor-zotero-full-{}-{}.sqlite",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        );
        let percorso = std::env::temp_dir().join(nome);
        assert!(!percorso.exists());
        let db = Connection::open(&percorso).unwrap();
        db.execute_batch(
            "CREATE TABLE collections(collectionID INTEGER, collectionName TEXT, parentCollectionID INTEGER);
             CREATE TABLE collectionItems(collectionID INTEGER, itemID INTEGER);
             CREATE TABLE items(itemID INTEGER, key TEXT, itemTypeID INTEGER, dateAdded TEXT);
             CREATE TABLE itemAttachments(itemID INTEGER, parentItemID INTEGER, contentType TEXT);
             CREATE TABLE itemTypes(itemTypeID INTEGER, typeName TEXT);
             CREATE TABLE itemData(itemID INTEGER, fieldID INTEGER, valueID INTEGER);
             CREATE TABLE fields(fieldID INTEGER, fieldName TEXT);
             CREATE TABLE itemDataValues(valueID INTEGER, value TEXT);
             CREATE TABLE deletedItems(itemID INTEGER);
             CREATE TABLE itemAnnotations(itemID INTEGER, parentItemID INTEGER, text TEXT, comment TEXT);
             CREATE TABLE deletedCollections(collectionID INTEGER);
             INSERT INTO collections VALUES(17, 'Esame Zotero aggiornato', NULL);
             INSERT INTO collectionItems VALUES(17, 100);
             INSERT INTO items VALUES(100, 'NEW', 1, '2026-11-04 10:00:00');
             INSERT INTO items VALUES(101, 'PDF', 2, '2026-11-04 10:00:00');
             INSERT INTO items VALUES(102, 'ANN', 3, '2026-11-04 10:00:00');
             INSERT INTO itemAttachments VALUES(101, 100, 'application/pdf');
             INSERT INTO itemTypes VALUES(1, 'book');
             INSERT INTO itemTypes VALUES(2, 'attachment');
             INSERT INTO itemTypes VALUES(3, 'annotation');
             INSERT INTO itemData VALUES(100, 1, 1);
             INSERT INTO fields VALUES(1, 'title');
             INSERT INTO itemDataValues VALUES(1, 'Documento nuovo');
             INSERT INTO itemAnnotations VALUES(102, 101, 'Annotazione nuova', 'Nota nuova');",
        )
        .unwrap();
        drop(db);
        percorso
    }

    fn stato_con_materiali_obsoleti() -> StatoApp {
        let mut stato = StatoApp::nuovo();
        stato.snapshot_zotero = Some(std::path::PathBuf::from("snapshot-precedente.sqlite"));
        stato.collezioni = vec![Collezione {
            id: 17,
            nome: "Esame precedente".into(),
            documenti: 1,
            sottocollezioni: 0,
        }];
        stato.esame_scelto = Some(17);
        stato.documenti_esame = vec![Documento {
            chiave: "OLD".into(),
            chiave_pdf: None,
            titolo: "Documento precedente".into(),
            tipo: "book".into(),
            data_aggiunta: String::new(),
        }];
        stato.testi_scelti = vec!["Documento precedente".into()];
        stato.evidenziazioni = vec![Evidenziazione {
            testo: "Annotazione precedente".into(),
            commento: String::new(),
            data: "2026-01-01".into(),
        }];
        stato.anteprima_notebooklm = Some(("old.md".into(), "old content".into()));
        stato.conferma = Some(("Materiali ricaricati".into(), std::time::Instant::now()));
        stato
    }

    #[test]
    fn errore_query_annotazioni_non_pubblica_documenti_parziali() {
        let mut stato = StatoApp::nuovo();
        stato.snapshot_zotero = Some(std::path::PathBuf::from("snapshot-precedente.sqlite"));
        stato.collezioni = vec![Collezione {
            id: 17,
            nome: "Esame".into(),
            documenti: 1,
            sottocollezioni: 0,
        }];
        stato.esame_scelto = Some(17);
        stato.documenti_esame = vec![Documento {
            chiave: "OLD".into(),
            chiave_pdf: None,
            titolo: "Documento precedente".into(),
            tipo: "book".into(),
            data_aggiunta: String::new(),
        }];
        stato.testi_scelti = vec!["Documento precedente".into()];
        stato.evidenziazioni = vec![Evidenziazione {
            testo: "Annotazione precedente".into(),
            commento: String::new(),
            data: "2026-01-01".into(),
        }];
        let snapshot = snapshot_senza_tabella_annotazioni();
        assert_eq!(
            zotero_query::documenti_collezione(&snapshot, 17, 60)
                .unwrap()
                .len(),
            1
        );
        assert!(zotero_query::evidenziazioni_esame(&snapshot, Some(17), 40).is_err());

        let risultato = applica_snapshot_materiali(&mut stato, 17, &snapshot);

        let _ = std::fs::remove_file(&snapshot);
        assert!(risultato.is_err());
        assert!(stato.snapshot_zotero.is_none());
        assert!(stato.collezioni.is_empty());
        assert!(stato.esame_scelto.is_none());
        assert!(stato.documenti_esame.is_empty());
        assert!(stato.testi_scelti.is_empty());
        assert!(stato.evidenziazioni.is_empty());
        assert!(stato.conferma.is_none());
    }

    #[test]
    fn errore_creazione_snapshot_invalida_i_dati_e_non_mostra_successo() {
        let mut stato = StatoApp::nuovo();
        stato.esame_scelto = Some(17);
        stato.snapshot_zotero = Some(std::path::PathBuf::from("snapshot-precedente.sqlite"));
        stato.collezioni = vec![Collezione {
            id: 17,
            nome: "Esame".into(),
            documenti: 1,
            sottocollezioni: 0,
        }];
        stato.documenti_esame.push(Documento {
            chiave: "OLD".into(),
            chiave_pdf: None,
            titolo: "Documento precedente".into(),
            tipo: "book".into(),
            data_aggiunta: String::new(),
        });
        stato.evidenziazioni.push(Evidenziazione {
            testo: "Annotazione precedente".into(),
            commento: String::new(),
            data: "2026-01-01".into(),
        });
        stato.config.cartella_zotero = std::env::temp_dir()
            .join(format!("zotero-missing-{}", std::process::id()))
            .to_string_lossy()
            .into_owned();
        stato.conferma = Some(("Testi ricaricati da Zotero.".into(), std::time::Instant::now()));

        ricarica_testi(&mut stato);

        assert!(stato.snapshot_zotero.is_none());
        assert!(stato.collezioni.is_empty());
        assert!(stato.documenti_esame.is_empty());
        assert!(stato.evidenziazioni.is_empty());
        assert!(stato.conferma.is_none());
        assert!(stato.avviso.is_some());
    }

    #[test]
    fn carica_esami_con_database_non_disponibile_invalida_stato_precedente() {
        let mut stato = stato_con_materiali_obsoleti();
        let root = cartella_app_test();
        stato.config.cartella_zotero = std::env::temp_dir()
            .join(format!("zotero-missing-load-{}", std::process::id()))
            .to_string_lossy()
            .into_owned();

        carica_esami_con(&mut stato, &root);

        let _ = std::fs::remove_dir_all(&root);
        assert!(stato.snapshot_zotero.is_none());
        assert!(stato.collezioni.is_empty());
        assert!(stato.esame_scelto.is_none());
        assert!(stato.documenti_esame.is_empty());
        assert!(stato.evidenziazioni.is_empty());
        assert!(stato.anteprima_notebooklm.is_none());
        assert!(stato.conferma.is_none());
        assert!(stato.avviso.is_some());
    }

    #[test]
    fn carica_esami_con_lista_vuota_non_mantiene_materiali_precedenti() {
        let mut stato = stato_con_materiali_obsoleti();
        let db = snapshot_vuoto();
        let root = cartella_app_test();
        stato.config.cartella_zotero = db.to_string_lossy().into_owned();

        carica_esami_con(&mut stato, &root);

        let _ = std::fs::remove_file(db);
        let _ = std::fs::remove_dir_all(&root);
        assert!(stato.snapshot_zotero.is_none());
        assert!(stato.collezioni.is_empty());
        assert!(stato.esame_scelto.is_none());
        assert!(stato.documenti_esame.is_empty());
        assert!(stato.evidenziazioni.is_empty());
        assert!(stato.anteprima_notebooklm.is_none());
        assert!(stato.conferma.is_none());
        assert!(stato
            .avviso
            .as_deref()
            .is_some_and(|a| a.to_lowercase().contains("nessun esame")));
    }

    #[test]
    fn errore_query_collezioni_invalida_stato_precedente() {
        let mut stato = stato_con_materiali_obsoleti();
        let db = snapshot_senza_schema();
        let root = cartella_app_test();
        stato.config.cartella_zotero = db.to_string_lossy().into_owned();

        carica_esami_con(&mut stato, &root);

        let _ = std::fs::remove_file(db);
        let _ = std::fs::remove_dir_all(&root);
        assert!(stato.snapshot_zotero.is_none());
        assert!(stato.collezioni.is_empty());
        assert!(stato.esame_scelto.is_none());
        assert!(stato.documenti_esame.is_empty());
        assert!(stato.evidenziazioni.is_empty());
        assert!(stato.anteprima_notebooklm.is_none());
        assert!(stato.conferma.is_none());
        assert!(stato.avviso.is_some());
    }

    #[test]
    fn cambia_esame_con_query_annotazioni_fallita_non_pubblica_documenti_parziali() {
        let mut stato = stato_con_materiali_obsoleti();
        let snapshot = snapshot_senza_tabella_annotazioni();
        stato.snapshot_zotero = Some(snapshot.clone());

        cambia_esame(&mut stato, Some(17));

        let _ = std::fs::remove_file(snapshot);
        assert!(stato.snapshot_zotero.is_none());
        assert!(stato.collezioni.is_empty());
        assert!(stato.esame_scelto.is_none());
        assert!(stato.documenti_esame.is_empty());
        assert!(stato.evidenziazioni.is_empty());
        assert!(stato.anteprima_notebooklm.is_none());
        assert!(stato.conferma.is_none());
        assert!(stato.avviso.as_deref().is_some_and(|a| a.contains("annotazioni")));
    }

    #[test]
    fn cambia_esame_riuscito_pulisce_toast_precedente() {
        let mut stato = stato_con_materiali_obsoleti();
        let snapshot = snapshot_con_annotazioni();
        stato.snapshot_zotero = Some(snapshot.clone());

        cambia_esame(&mut stato, Some(17));

        let _ = std::fs::remove_file(snapshot);
        assert!(stato.conferma.is_none());
        assert_eq!(stato.esame_scelto, Some(17));
        assert_eq!(stato.documenti_esame[0].titolo, "Documento nuovo");
    }

    #[test]
    fn deselezionare_esame_pulisce_toast_precedente() {
        let mut stato = stato_con_materiali_obsoleti();

        cambia_esame(&mut stato, None);

        assert!(stato.conferma.is_none());
        assert!(stato.esame_scelto.is_none());
    }

    #[test]
    fn refresh_completo_aggiorna_documenti_annotazioni_e_solo_selezioni_valide() {
        let mut stato = stato_con_materiali_obsoleti();
        stato.testi_scelti.push("Documento nuovo".into());
        let db = snapshot_con_annotazioni();
        stato.config.cartella_zotero = db.to_string_lossy().into_owned();
        let root = std::env::temp_dir().join(format!(
            "focusanchor-zotero-refresh-{}-{}",
            std::process::id(),
            SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos()
        ));
        assert!(!root.exists());
        std::fs::create_dir(&root).unwrap();

        let risultato = aggiorna_materiali_zotero(&mut stato, 17, &root, 400);

        let _ = std::fs::remove_file(&db);
        let _ = std::fs::remove_dir_all(&root);
        assert!(risultato.is_ok());
        assert_eq!(stato.documenti_esame.len(), 1);
        assert_eq!(stato.documenti_esame[0].titolo, "Documento nuovo");
        assert_eq!(stato.evidenziazioni.len(), 1);
        assert_eq!(stato.evidenziazioni[0].testo, "Annotazione nuova");
        assert_eq!(stato.testi_scelti, vec!["Documento nuovo"]);
        assert_eq!(stato.esame_scelto, Some(17));
        assert_eq!(stato.collezioni[0].nome, "Esame Zotero aggiornato");
        assert!(stato.snapshot_zotero.is_some());
    }
}
