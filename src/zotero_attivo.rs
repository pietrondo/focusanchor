//! Cosa l'estensione Zotero dice che stai guardando adesso.
//!
//! L'app indovina il documento dal titolo della finestra, e sbaglia: la
//! finestra tronca i titoli lunghi, un PDF aperto in una seconda finestra non
//! si vede, e due titoli simili diventano lo stesso documento. L'estensione
//! scrive la chiave esatta dell'item, quindi qui non c'e' niente da indovinare.
//!
//! Il file e' ocusanchor_attivo.json nella cartella dati di Zotero, dove
//! c'e' gia' il database: l'app lo trova senza configurazione.

use std::fs;
use std::path::PathBuf;

use crate::zotero::{percorso_database_personalizzato, Documento};

/// Cosa l'estensione dice che stai guardando adesso.
///
/// E' l'alternativa esatta al titolo della finestra: la finestra tronca i
/// titoli lunghi e un PDF aperto in una seconda finestra non si vede.
#[derive(serde::Deserialize, Clone, Debug)]
pub struct Attivo {
    /// Chiave dell'item aperto (per un PDF, quella dell'allegato).
    pub chiave: String,
    /// Chiave dell'item che contiene l'allegato.
    #[serde(default, rename = "chiavePadre")]
    pub chiave_padre: String,
    /// Titolo leggibile.
    #[serde(default)]
    pub titolo: String,
}

/// Percorso del file scritto dall'estensione Zotero.
pub fn percorso_attivo(cartella_config: &str) -> Option<PathBuf> {
    percorso_database_personalizzato(cartella_config)
        .and_then(|db| db.parent().map(|dir| dir.join("focusanchor_attivo.json")))
}

/// Legge il documento attivo, se l'estensione l'ha scritto da poco.
///
/// La freschezza conta: con Zotero chiuso il file resta li' e direbbe una
/// bugia su cosa stai guardando.
pub fn leggi_attivo(cartella_config: &str) -> Option<Attivo> {
    /// Oltre questo silenzio si considera l'informazione vecchia.
    const FRESCHEZZA: std::time::Duration = std::time::Duration::from_secs(30);

    let percorso = percorso_attivo(cartella_config)?;
    let scritto = fs::metadata(&percorso).ok()?.modified().ok()?;
    if scritto.elapsed().ok()? > FRESCHEZZA {
        return None;
    }
    let testo = fs::read_to_string(&percorso).ok()?;
    serde_json::from_str::<Option<Attivo>>(&testo).ok().flatten()
}

/// Titolo del documento che l'estensione dice essere aperto.
///
/// Il confronto e' per chiave, non per somiglianza: e' il motivo per cui
/// l'estensione esiste.
pub fn classifica_attivo(attivo: &Attivo, documenti: &[Documento]) -> Option<String> {
    documenti
        .iter()
        .find(|documento| {
            documento.chiave == attivo.chiave
                || documento.chiave == attivo.chiave_padre
                || documento.chiave_pdf.as_deref() == Some(attivo.chiave.as_str())
        })
        .map(|documento| documento.titolo.clone())
}
