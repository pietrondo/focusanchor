//! Query sul database Zotero: collezioni, documenti e sottolineature.
//!
//! Ogni query lavora sullo snapshot creato da `zotero::crea_snapshot_con`,
//! aperto in sola lettura: Zotero puo' restare aperto senza conflitti.

use std::path::Path;

use rusqlite::Connection;

use crate::zotero::{Collezione, Documento, Evidenziazione};

/// Sottoquery che estrae l'identificativo del campo "title".
const CAMPO_TITOLO: &str = "(SELECT fieldID FROM fields WHERE fieldName='title')";

/// Apre in sola lettura lo snapshot del database Zotero.
pub fn apri_snapshot(snapshot: &Path) -> Result<Connection, String> {
    Connection::open_with_flags(
        snapshot,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY | rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .map_err(|e| format!("Apertura database Zotero: {e}"))
}

/// Elenco delle collezioni, cioe' gli esami da preparare.
///
/// Il conteggio esclude gli item nel cestino: altrimenti una collezione
/// svuotata mostrerebbe "7 documenti" e poi una lista vuota.
pub fn collezioni(snapshot: &Path) -> Result<Vec<Collezione>, String> {
    let db = apri_snapshot(snapshot)?;
    let query = "
        SELECT c.collectionID, c.collectionName,
               (SELECT COUNT(DISTINCT COALESCE(ia.parentItemID, ci.itemID))
                FROM collectionItems ci
                JOIN items i ON i.itemID = ci.itemID
                LEFT JOIN itemAttachments ia ON ia.itemID = i.itemID
                WHERE ci.collectionID = c.collectionID
                  AND COALESCE(ia.parentItemID, ci.itemID)
                      NOT IN (SELECT itemID FROM deletedItems))
        FROM collections c
        ORDER BY c.collectionName";
    let mut stmt = db.prepare(query).map_err(|e| e.to_string())?;
    let righe = stmt
        .query_map([], |r| {
            Ok(Collezione {
                id: r.get(0)?,
                nome: r.get(1)?,
                documenti: r.get::<_, i64>(2)? as usize,
            })
        })
        .map_err(|e| e.to_string())?;

    let mut risultati = Vec::new();
    for riga in righe {
        let collezione = riga.map_err(|e| e.to_string())?;
        if !collezione.nome.trim().is_empty() {
            risultati.push(collezione);
        }
    }
    Ok(risultati)
}

/// Documenti di una collezione, con il titolo del loro item padre.
///
/// La collezione contiene sia i PDF sia gli item che li contengono: si prende
/// sempre il padre, cosi' l'utente vede il titolo del testo e non "PDF".
pub fn documenti_collezione(
    snapshot: &Path,
    collezione: i64,
    limite: usize,
) -> Result<Vec<Documento>, String> {
    let db = apri_snapshot(snapshot)?;
    let query = format!(
        "
        SELECT DISTINCT p.key, v.value, it.typeName, p.dateAdded
        FROM collectionItems ci
        JOIN items i ON i.itemID = ci.itemID
        LEFT JOIN itemAttachments ia ON ia.itemID = i.itemID
        JOIN items p ON p.itemID = COALESCE(ia.parentItemID, i.itemID)
        JOIN itemTypes it ON it.itemTypeID = p.itemTypeID
        JOIN itemData d ON d.itemID = p.itemID AND d.fieldID = {CAMPO_TITOLO}
        JOIN itemDataValues v ON v.valueID = d.valueID
        WHERE ci.collectionID = ?1
          AND p.itemID NOT IN (SELECT itemID FROM deletedItems)
        ORDER BY p.dateAdded DESC
        LIMIT ?2"
    );
    let mut stmt = db.prepare(&query).map_err(|e| e.to_string())?;
    let righe = stmt
        .query_map([collezione, limite as i64], |r| {
            Ok(Documento {
                chiave: r.get(0)?,
                titolo: r.get(1)?,
                tipo: r.get(2)?,
                data_aggiunta: r.get(3)?,
            })
        })
        .map_err(|e| e.to_string())?;

    let mut risultati = Vec::new();
    for riga in righe {
        let doc = riga.map_err(|e| e.to_string())?;
        if !doc.titolo.trim().is_empty() {
            risultati.push(doc);
        }
    }
    Ok(risultati)
}

/// Estrae le ultime sottolineature, eventualmente filtrate per collezione.
///
/// Il filtro per esame e' quello che rende utile l'interrogazione: le domande
/// a tradimento devono nascere dai testi che l'utente sta studiando adesso,
/// non da un altro corso.
pub fn evidenziazioni_esame(
    snapshot: &Path,
    collezione: Option<i64>,
    limite: usize,
) -> Result<Vec<Evidenziazione>, String> {
    let db = apri_snapshot(snapshot)?;
    // Il legame e': annotazione -> attachment (padre) -> item nella collezione.
    let query = "
        SELECT a.text, a.comment, i.dateAdded
        FROM itemAnnotations a
        JOIN items i ON i.itemID = a.itemID
        JOIN itemAttachments ia ON ia.itemID = a.parentItemID
        WHERE a.text IS NOT NULL AND a.text != ''
          AND (
            ?1 IS NULL
            OR ia.parentItemID IN (SELECT itemID FROM collectionItems WHERE collectionID = ?1)
            OR ia.itemID IN (SELECT itemID FROM collectionItems WHERE collectionID = ?1)
          )
        ORDER BY a.itemID DESC
        LIMIT ?2";
    let mut stmt = db.prepare(query).map_err(|e| e.to_string())?;
    let righe = stmt
        .query_map(rusqlite::params![collezione, limite as i64], |r| {
            Ok(Evidenziazione {
                testo: r.get(0)?,
                commento: r.get::<_, Option<String>>(1)?.unwrap_or_default(),
                data: data_giorno(&r.get::<_, Option<String>>(2)?),
            })
        })
        .map_err(|e| e.to_string())?;

    let mut risultati = Vec::new();
    for riga in righe {
        risultati.push(riga.map_err(|e| e.to_string())?);
    }
    Ok(risultati)
}

/// Estrae la data (YYYY-MM-DD) da un timestamp di Zotero.
///
/// Il formato memorizzato e' `YYYY-MM-DD HH:MM:SS`.
fn data_giorno(timestamp: &Option<String>) -> String {
    let testo = timestamp.as_deref().unwrap_or("");
    if testo.len() < 10 {
        return String::new();
    }
    testo[..10].to_string()
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn estrae_la_data_dal_timestamp_di_zotero() {
        assert_eq!(
            data_giorno(&Some("2026-10-27 14:02:46".to_string())),
            "2026-10-27"
        );
        assert_eq!(data_giorno(&None), "");
        assert_eq!(data_giorno(&Some("corto".to_string())), "");
    }

    /// Test reale sul database Zotero dell'utente.
    ///
    /// Ignorato di proposito: richiede Zotero installato, e non deve fallire
    /// la build su un'altra macchina. Si esegue con:
    /// `cargo test zotero -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn leggere_collezioni_e_annotazioni_reali() {
        let cartella = std::env::temp_dir().join("focusanchor_zotero_test");
        let snapshot = crate::zotero::crea_snapshot_con(&cartella, "").expect("snapshot");
        assert!(snapshot.exists());

        let elenco = collezioni(&snapshot).expect("query collezioni");
        println!("collezioni: {}", elenco.len());
        for c in elenco.iter().take(8) {
            println!("  [{}] {} ({} documenti)", c.id, c.nome, c.documenti);
        }
        assert!(!elenco.is_empty(), "l'utente ha almeno una collezione");

        // Ogni collezione con documenti deve restituirli davvero: conteggio
        // e lista devono restare coerenti (era il bug dei cestinati).
        for c in elenco.iter().filter(|c| c.documenti > 0).take(3) {
            let documenti =
                documenti_collezione(&snapshot, c.id, 20).expect("documenti collezione");
            println!("documenti di '{}': {}", c.nome, documenti.len());
            for d in documenti.iter().take(3) {
                println!("   - {}", d.titolo);
            }
        }

        let tutte = evidenziazioni_esame(&snapshot, None, 10).expect("evidenze globali");
        println!("sottolineature totali (limite 10): {}", tutte.len());
        assert!(!tutte.is_empty(), "ci sono annotazioni nella libreria");
    }
}
