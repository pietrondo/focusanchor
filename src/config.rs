//! Configurazione persistente dell'applicazione (`config.json`).

use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// Configurazione salvata su disco accanto all'eseguibile.
#[derive(Serialize, Deserialize, Clone)]
pub struct Configurazione {
    /// L'unica cosa da studiare nella sessione corrente.
    pub obiettivo: String,
    /// Durata della sessione in minuti.
    pub durata_minuti: u32,
    /// Modalita' di audio di sottofondo scelta dall'utente.
    pub modalita_audio: String,
    /// Volume del suono di sottofondo (0.0 - 1.0).
    pub volume: f32,
    /// Velocita' di lettura della voce.
    pub velocita_voce: String,
    /// Richiamo vocale ogni 15 minuti attivo.
    pub richiamo_vocale: bool,
    /// Voce attiva: quando e' spenta l'app non parla mai.
    ///
    /// Il rumore di sottofondo e la voce escono dallo stesso altoparlante:
    /// con il rumore acceso, la voce sembra "spuntare" dal rumore. Chi
    /// preferisce silenzio totale spegne tutto da qui.
    #[serde(default = "vero")]
    pub voce_attiva: bool,
    /// Sfide anti-noia a sorpresa attive.
    pub sfide_a_sorpresa: bool,
    /// Domini bloccati nel file `hosts`.
    pub domini_vietati: Vec<String>,
    /// Parole chiave cercate nei titoli delle finestre.
    pub parole_vietate: Vec<String>,
    /// Cartella Zotero personalizzata (vuota = percorso standard).
    pub cartella_zotero: String,
    /// Link del notebook NotebookLM di ogni esame, per nome.
    ///
    /// Ogni esame ha il suo notebook con i testi caricati: salvando il link
    /// una volta, "Spiega con NotebookLM" apre quello giusto invece della
    /// pagina iniziale.
    #[serde(default)]
    pub notebook_esami: std::collections::BTreeMap<String, String>,
}

impl Default for Configurazione {
    fn default() -> Self {
        Self {
            obiettivo: String::new(),
            durata_minuti: 25,
            modalita_audio: "RumoreMarrone".to_string(),
            volume: 0.35,
            velocita_voce: "Normale".to_string(),
            richiamo_vocale: true,
            voce_attiva: true,
            sfide_a_sorpresa: true,
            domini_vietati: default_domini(),
            parole_vietate: default_parole(),
            cartella_zotero: String::new(),
            notebook_esami: std::collections::BTreeMap::new(),
        }
    }
}

/// Valore di default `true`, usato da serde quando il campo manca.
fn vero() -> bool {
    true
}

/// Domini bloccati per default (social e video che fanno perdere il filo).
pub fn default_domini() -> Vec<String> {
    [
        "reddit.com",
        "www.reddit.com",
        "old.reddit.com",
        "youtube.com",
        "www.youtube.com",
        "x.com",
        "twitter.com",
        "facebook.com",
        "instagram.com",
        "twitch.tv",
    ]
    .iter()
    .map(|s| s.to_string())
    .collect()
}

/// Parole chiave cercate nei titoli delle finestre attive.
pub fn default_parole() -> Vec<String> {
    [
        "reddit",
        " - youtube",
        "twitter",
        " / x",
        "facebook",
        "instagram",
        "twitch",
    ]
    .iter()
    .map(|s| s.to_string())
    .collect()
}

/// Percorso della cartella dati (stessa cartella dell'eseguibile).
pub fn cartella_dati() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.to_path_buf()))
        .unwrap_or_else(|| PathBuf::from("."))
}

/// Percorso del file di configurazione.
pub fn percorso_configurazione() -> PathBuf {
    cartella_dati().join("config.json")
}

/// Carica la configurazione da disco, con i valori di default in caso di errore.
pub fn carica_configurazione() -> Configurazione {
    let percorso = percorso_configurazione();
    let Ok(testo) = fs::read_to_string(&percorso) else {
        return Configurazione::default();
    };
    serde_json::from_str(&testo).unwrap_or_default()
}

/// Salva la configurazione su disco in formato leggibile.
pub fn salva_configurazione(config: &Configurazione) {
    let percorso = percorso_configurazione();
    if let Ok(json) = serde_json::to_string_pretty(config) {
        let _ = fs::write(percorso, json);
    }
}

/// Converte un elenco di righe in una lista pulita di voci.
pub fn righe_alla_lista(testo: &str) -> Vec<String> {
    testo.lines()
        .map(|riga| riga.trim().to_string())
        .filter(|riga| !riga.is_empty())
        .collect()
}

/// Converte una lista di voci in testo con una voce per riga.
pub fn lista_alle_righe(lista: &[String]) -> String {
    lista.join("\n")
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn ignora_le_righe_vuote_nelle_liste() {
        let lista = righe_alla_lista("reddit.com\n\n  youtube.com  \n");
        assert_eq!(lista, vec!["reddit.com", "youtube.com"]);
    }
}
