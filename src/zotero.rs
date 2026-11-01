//! Integrazione nativa con Zotero: percorsi, snapshot e tipi.
//!
//! Legge il database locale `zotero.sqlite` tramite uno snapshot sicuro:
//! il file viene copiato (con i suoi file WAL e SHM) in una cartella
//! temporanea, cosi' l'originale non viene mai aperto e Zotero puo' continuare
//! a lavorarci senza blocchi.
//!
//! Le query sono in `zotero_query.rs`.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Documento estratto dalla libreria Zotero.
#[derive(Clone, Debug)]
pub struct Documento {
    /// Chiave interna dell'oggetto Zotero.
    pub chiave: String,
    /// Titolo del documento.
    pub titolo: String,
    /// Tipo di oggetto (journalArticle, book, attachment...).
    pub tipo: String,
    /// Data di aggiunta, in formato Zotero.
    pub data_aggiunta: String,
}

/// Collezione Zotero, cioe' un esame da preparare.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Collezione {
    /// Identificativo interno della collezione.
    pub id: i64,
    /// Nome dell'esame o della collezione.
    pub nome: String,
    /// Numero di documenti contenuti.
    pub documenti: usize,
}

/// Sottolineatura o nota fatta dall'utente con il pennarello.
#[derive(Clone, Debug)]
pub struct Evidenziazione {
    /// Testo evidenziato nel PDF.
    pub testo: String,
    /// Commento dell'utente (se presente).
    pub commento: String,
    /// Data della sottolineatura, in formato Zotero.
    pub data: String,
}

/// Percorso della cartella dati di Zotero.
pub fn cartella_zotero() -> Option<PathBuf> {
    dirs::home_dir().map(|casa| casa.join("Zotero"))
}

/// Percorso del database Zotero, se la cartella standard esiste.
pub fn percorso_database() -> Option<PathBuf> {
    cartella_zotero().map(|c| c.join("zotero.sqlite"))
}

/// Percorso del database Zotero, rispettando un percorso personalizzato.
///
/// Se l'utente ha indicato a mano la propria cartella Zotero, si usa quella:
/// chi ha spostato i dati o usa un profilo diverso la ritrova cosi'.
pub fn percorso_database_personalizzato(cartella: &str) -> Option<PathBuf> {
    let personalizzato = cartella.trim();
    if !personalizzato.is_empty() {
        let percorso = PathBuf::from(personalizzato);
        // Si accetta sia la cartella sia il file zotero.sqlite.
        if percorso.is_dir() {
            return Some(percorso.join("zotero.sqlite"));
        }
        if percorso.extension().is_some() {
            return Some(percorso);
        }
        return Some(percorso.join("zotero.sqlite"));
    }
    percorso_database()
}

/// Crea uno snapshot del database Zotero in una cartella temporanea.
///
/// Usa l'API di backup di SQLite: legge il database vivo (WAL compreso) in
/// modo coerente, senza le corse che si hanno copiando i file a mano mentre
/// Zotero scrive. Se il backup non e' possibile, ripiega sulla copia dei file.
pub fn crea_snapshot_con(cartella_app: &Path, cartella_zotero: &str) -> Result<PathBuf, String> {
    let sorgente = percorso_database_personalizzato(cartella_zotero)
        .ok_or("Cartella utente non trovata.")?;
    if !sorgente.exists() {
        return Err(format!(
            "Non trovo il database Zotero in {}. Apri Zotero e riprova.",
            sorgente.display()
        ));
    }
    let destinazione = cartella_app.join("zotero_snapshot");
    let _ = fs::remove_dir_all(&destinazione);
    fs::create_dir_all(&destinazione).map_err(|e| format!("Cartella snapshot: {e}"))?;
    let file = destinazione.join("zotero.sqlite");

    match backup_coerente(&sorgente, &file) {
        Ok(()) => Ok(file),
        // Se Zotero tiene un lock esclusivo nell'istante sbagliato, copiare i
        // file resta meglio che non avere dati.
        Err(_) => copia_file(&sorgente, &destinazione).map(|_| file),
    }
}

/// Copia il database con l'API di backup di SQLite.
///
/// Dopo il backup il file viene portato in modalita' journal classica: cosi'
/// lo snapshot e' un singolo file autosufficiente, senza WAL da portarsi
/// dietro in sola lettura.
///
/// Il backup e' a tempo: se Zotero tiene un lock di scrittura, si rinuncia
/// dopo `ATTESA_MASSIMA` invece di bloccare l'interfaccia per sempre.
fn backup_coerente(sorgente: &Path, destinazione: &Path) -> Result<(), String> {
    use rusqlite::backup::StepResult;
    use rusqlite::OpenFlags;
    use std::time::{Duration, Instant};

    /// Quanto si aspetta un lock di Zotero prima di ripiegare sulla copia.
    const ATTESA_MASSIMA: Duration = Duration::from_secs(5);

    let src = rusqlite::Connection::open_with_flags(
        sorgente,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .map_err(|e| format!("Apertura Zotero: {e}"))?;
    let mut dst = rusqlite::Connection::open(destinazione)
        .map_err(|e| format!("Creazione snapshot: {e}"))?;

    {
        let backup = rusqlite::backup::Backup::new(&src, &mut dst)
            .map_err(|e| format!("Backup Zotero: {e}"))?;
        let scadenza = Instant::now() + ATTESA_MASSIMA;
        loop {
            match backup
                .step(256)
                .map_err(|e| format!("Copia dati Zotero: {e}"))?
            {
                StepResult::Done => break,
                StepResult::More => {}
                StepResult::Busy | StepResult::Locked => {
                    if Instant::now() >= scadenza {
                        return Err("Zotero sta scrivendo: riprova fra un istante.".into());
                    }
                    std::thread::sleep(Duration::from_millis(40));
                }
                // `StepResult` non e' esaustivo per il compilatore: ogni
                // variante futura va trattata come "riprova".
                _ => {
                    if Instant::now() >= scadenza {
                        return Err("Backup Zotero non riuscito in tempo.".into());
                    }
                    std::thread::sleep(Duration::from_millis(40));
                }
            }
        }
    }

    let _ = dst.execute_batch("PRAGMA journal_mode=DELETE;");
    Ok(())
}

/// Copia i file del database (main, WAL, SHM) come ripiego.
fn copia_file(sorgente: &Path, destinazione: &Path) -> Result<(), String> {
    let copia = |nome: &str| -> Result<(), String> {
        let da = sorgente.with_file_name(nome);
        if da.exists() {
            fs::copy(&da, destinazione.join(nome)).map_err(|e| format!("Copia di {nome}: {e}"))?;
        }
        Ok(())
    };
    copia("zotero.sqlite")?;
    copia("zotero.sqlite-wal")?;
    copia("zotero.sqlite-shm")?;
    Ok(())
}

/// Apre un oggetto di Zotero con il suo protocollo `zotero://`.
///
/// Permette di portare l'utente dritto al documento selezionato, senza
/// doverlo cercare a mano nella libreria.
pub fn apri_in_zotero(chiave: &str) -> Result<(), String> {
    use std::os::windows::process::CommandExt;
    let url = format!("zotero://select/items/{chiave}");
    Command::new("cmd")
        .args(["/C", "start", "", &url])
        .creation_flags(crate::blocker::CREATE_NO_WINDOW)
        .spawn()
        .map(|_| ())
        .map_err(|e| format!("Non riesco ad aprire Zotero: {e}"))
}

/// Normalizza un titolo per confrontarlo con il titolo di una finestra.
///
/// Le finestre troncano i titoli lunghi e i PDF usano spesso caratteri
/// tipografici diversi: si riduce tutto a lettere e cifre minuscole.
pub fn normalizza(titolo: &str) -> String {
    titolo
        .to_lowercase()
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == ' ')
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// Vero se il titolo della finestra Zotero parla di uno dei testi dell'esame.
///
/// Il confronto e' tollerante: la finestra puo' troncare il titolo, quindi si
/// accetta che uno contenga l'altro, purche' il frammento sia abbastanza
/// lungo da non generare falsi positivi.
pub fn appartiene_all_esame(nome_finestra: &str, testi_esame: &[String]) -> bool {
    let finestra = normalizza(nome_finestra);
    if finestra.len() < 8 {
        return false;
    }
    testi_esame.iter().any(|testo| {
        let atteso = normalizza(testo);
        if atteso.len() < 8 {
            return false;
        }
        finestra.contains(&atteso) || atteso.contains(&finestra)
    })
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn il_percorso_personalizzato_viene_usato() {
        let personalizzato = percorso_database_personalizzato("C:\\Users\\test\\Zotero");
        assert!(personalizzato.unwrap().ends_with("zotero.sqlite"));
        assert!(percorso_database_personalizzato("").is_some());
    }

    #[test]
    fn riconosce_il_testo_dell_esame_anche_se_troncato() {
        let testi = vec!["Storia economica riassunto".to_string()];
        assert!(appartiene_all_esame("Storia economica riassunto", &testi));
        // La finestra puo' troncare il titolo.
        assert!(appartiene_all_esame("Storia economica", &testi));
        // Un altro documento non deve passare.
        assert!(!appartiene_all_esame("Diritto privato", &testi));
    }

    #[test]
    fn i_titoli_troppo_corti_non_generano_falsi_positivi() {
        let testi = vec!["PDF".to_string()];
        assert!(!appartiene_all_esame("PDF", &testi));
        assert!(!appartiene_all_esame("Qualsiasi altra cosa", &testi));
    }

    #[test]
    fn la_normalizzazione_ignora_maiuscole_e_punteggiatura() {
        assert_eq!(normalizza("  Storia  Economica!  "), "storia economica");
    }
}
