//! Comandi inviati dall'estensione Zotero all'app.
//!
//! Il punto di partenza e' sempre Zotero: aprire una seconda finestra solo per
//! premere "avvia" e' il tipo di attrito che fa abbandonare lo studio. Qui
//! l'estensione scrive un file di comando nella cartella dati di Zotero, che
//! l'app legge una volta al secondo.
//!
//! Niente porte, niente server, niente chiavi: un file in una cartella che
//! entrambi conoscono gia'. Se l'app e' spenta, il plugin la avvia prima di
//! scrivere, e il comando resta in attesa finche' lei non lo consuma.

use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// Un comando arrivato da Zotero.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Comando {
    /// Identificativo unico: serve a non eseguire due volte lo stesso file.
    pub id: String,
    /// `avvia`, `pausa`, `riprendi`, `ferma`, `ripasso`.
    pub comando: String,
    /// Esame da studiare, per il comando `avvia`.
    #[serde(default)]
    pub esame: String,
}

/// Cartella condivisa con l'estensione: quella dati di Zotero.
fn cartella(cartella_config: &str) -> Option<PathBuf> {
    crate::zotero::percorso_database_personalizzato(cartella_config)
        .and_then(|db| db.parent().map(|dir| dir.to_path_buf()))
}

/// Percorso del file di comando.
pub fn percorso_comando(cartella_config: &str) -> Option<PathBuf> {
    cartella(cartella_config).map(|dir| dir.join("focusanchor_comando.json"))
}

/// Percorso del battito con cui l'app dice di essere viva.
pub fn percorso_battito(cartella_config: &str) -> Option<PathBuf> {
    cartella(cartella_config).map(|dir| dir.join("focusanchor_vivo.json"))
}

/// Percorso in cui l'app lascia la propria posizione, per farsi riavviare.
pub fn percorso_eseguibile(cartella_config: &str) -> Option<PathBuf> {
    cartella(cartella_config).map(|dir| dir.join("focusanchor_exe.txt"))
}

/// Legge il comando in attesa, se c'e'.
pub fn leggi(cartella_config: &str) -> Option<Comando> {
    let percorso = percorso_comando(cartella_config)?;
    let testo = fs::read_to_string(percorso).ok()?;
    serde_json::from_str(&testo).ok()
}

/// Cancella il comando dopo averlo eseguito.
///
/// Il file sparisce invece di essere marcato: cosi' non c'e' stato da
/// mantenere e un riavvio non riesegue niente.
pub fn consuma(cartella_config: &str) {
    if let Some(percorso) = percorso_comando(cartella_config) {
        let _ = fs::remove_file(percorso);
    }
}

/// Dice a Zotero che l'app e' viva, cosa sta facendo e dove si trova.
///
/// Il battito serve al plugin per due cose: decidere se avviare l'app (un file
/// fermo significa che e' chiusa) e riempire il pannello dentro Zotero senza
/// che l'utente debba cambiare finestra.
pub fn battito(stato: &crate::state::StatoApp) {
    #[derive(Serialize)]
    struct Battito {
        pid: u32,
        eseguibile: String,
        #[serde(rename = "inSessione")]
        in_sessione: bool,
        #[serde(rename = "inPausa")]
        in_pausa: bool,
        secondi: u64,
        esame: String,
        #[serde(rename = "daRipassare")]
        da_ripassare: usize,
        aggiornato: u64,
    }

    let Some(percorso) = percorso_battito(&stato.config.cartella_zotero) else {
        return;
    };
    let eseguibile = std::env::current_exe().unwrap_or_default();
    let oggi = crate::carte::oggi();
    let battito = Battito {
        pid: std::process::id(),
        eseguibile: eseguibile.display().to_string(),
        in_sessione: stato.sessione.is_some(),
        in_pausa: stato.pausa_residua().is_some(),
        secondi: stato.secondi_sessione(),
        esame: stato.nome_esame(),
        da_ripassare: stato.memoria.scadute(&oggi).len(),
        aggiornato: chrono::Local::now().timestamp_millis() as u64,
    };
    if let Ok(json) = serde_json::to_string(&battito) {
        let _ = fs::write(percorso, json);
    }
    if let Some(dove) = percorso_eseguibile(&stato.config.cartella_zotero) {
        let _ = fs::write(dove, eseguibile.display().to_string());
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn un_comando_si_legge_e_si_consuma() {
        let cartella = std::env::temp_dir().join("focusanchor_comando_test");
        let _ = fs::create_dir_all(&cartella);
        let db = cartella.join("zotero.sqlite");
        fs::write(&db, b"").unwrap();

        let percorso = percorso_comando(&cartella.to_string_lossy()).expect("percorso");
        fs::write(
            &percorso,
            r#"{"id":"abc","comando":"avvia","esame":"storia romana"}"#,
        )
        .unwrap();

        let comando = leggi(&cartella.to_string_lossy()).expect("comando");
        assert_eq!(comando.comando, "avvia");
        assert_eq!(comando.esame, "storia romana");

        consuma(&cartella.to_string_lossy());
        assert!(leggi(&cartella.to_string_lossy()).is_none());
    }

    #[test]
    fn un_file_rotto_non_fa_esplodere_niente() {
        let cartella = std::env::temp_dir().join("focusanchor_comando_rotto");
        let _ = fs::create_dir_all(&cartella);
        fs::write(cartella.join("zotero.sqlite"), b"").unwrap();
        let percorso = percorso_comando(&cartella.to_string_lossy()).expect("percorso");
        fs::write(&percorso, "{non json").unwrap();
        assert!(leggi(&cartella.to_string_lossy()).is_none());
    }
}
