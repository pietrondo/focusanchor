//! Configurazione persistente dell'applicazione (`config.json`).

use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// Scheda di un esame: le informazioni che servono per prepararlo.
#[derive(Serialize, Deserialize, Clone, Default)]
pub struct InfoEsame {
    /// Pagina del corso su Unimi o altro ateneo.
    pub url: String,
    /// Crediti formativi.
    pub crediti: u32,
    /// Modalita' d'esame: "Scritto", "Orale" o entrambi.
    pub modalita: String,
    /// Vero se il programma e' da non frequentante.
    pub non_frequentante: bool,
    /// Data dell'appello a cui si punta, formato `AAAA-MM-GG`.
    pub data_esame: String,
}

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
    /// Scheda di ogni esame: link al corso, crediti, modalita', data.
    #[serde(default)]
    pub esami: std::collections::BTreeMap<String, InfoEsame>,
    /// Punti XP accumulati: sopravvivono alla chiusura dell'app.
    #[serde(default)]
    pub xp_punti: i32,
    /// Distrazioni sventate in totale.
    #[serde(default)]
    pub xp_distrazioni: u32,
    /// Modello usato per l'assistente AI, in formato `provider/modello`.
    #[serde(default = "modello_ai_default")]
    pub modello_ai: String,
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
            esami: std::collections::BTreeMap::new(),
            xp_punti: 0,
            xp_distrazioni: 0,
            modello_ai: modello_ai_default(),
        }
    }
}

/// Valore di default `true`, usato da serde quando il campo manca.
fn vero() -> bool {
    true
}

/// Modello AI predefinito: quello gia' configurato in OpenCode.
fn modello_ai_default() -> String {
    "opencode-go/deepseek-v4.1-flash".to_string()
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
    let _ = salva_configurazione_con_esito(config);
}

/// Salva la configurazione e restituisce l'errore se serializzazione o scrittura falliscono.
pub fn salva_configurazione_con_esito(config: &Configurazione) -> Result<(), String> {
    salva_configurazione_a(config, &percorso_configurazione())
}

fn salva_configurazione_a(config: &Configurazione, percorso: &std::path::Path) -> Result<(), String> {
    salva_configurazione_a_con_writer(config, percorso, |file, bytes| file.write_all(bytes))
}

fn salva_configurazione_a_con_writer(
    config: &Configurazione,
    percorso: &std::path::Path,
    scrivi: impl FnOnce(&mut File, &[u8]) -> io::Result<()>,
) -> Result<(), String> {
    let json = serde_json::to_vec_pretty(config)
        .map_err(|e| format!("Serializzazione configurazione fallita: {e}"))?;
    let directory = percorso.parent().unwrap_or_else(|| std::path::Path::new("."));
    let nome = percorso
        .file_name()
        .ok_or_else(|| "Percorso configurazione non valido".to_string())?
        .to_string_lossy();
    let temporaneo = directory.join(format!(
        ".{nome}.{}.{}.tmp",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|e| format!("Orologio di sistema non valido: {e}"))?
            .as_nanos()
    ));

    let mut temporaneo_creato = false;
    let risultato = (|| {
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporaneo)
            .map_err(|e| format!("Creazione file temporaneo configurazione fallita: {e}"))?;
        temporaneo_creato = true;
        scrivi(&mut file, &json)
            .map_err(|e| format!("Scrittura configurazione temporanea fallita: {e}"))?;
        file.sync_all()
            .map_err(|e| format!("Sincronizzazione configurazione temporanea fallita: {e}"))?;
        drop(file);
        fs::rename(&temporaneo, percorso)
            .map_err(|e| format!("Sostituzione configurazione fallita: {e}"))?;
        Ok(())
    })();

    if risultato.is_err() && temporaneo_creato {
        let _ = fs::remove_file(&temporaneo);
    }
    risultato
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
    fn errore_scrittura_configurazione_viene_propagato() {
        let directory = std::env::temp_dir().join(format!(
            "focusanchor-config-write-failure-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&directory);
        fs::create_dir(&directory).unwrap();

        let result = salva_configurazione_a(&Configurazione::default(), &directory);

        let _ = fs::remove_dir_all(&directory);
        assert!(result.is_err());
    }

    #[test]
    fn errore_dopo_scrittura_parziale_sul_temporaneo_preserva_config_esistente() {
        let directory = std::env::temp_dir().join(format!(
            "focusanchor-config-atomic-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&directory);
        fs::create_dir(&directory).unwrap();
        let destinazione = directory.join("config.json");
        fs::write(&destinazione, "config precedente integra").unwrap();

        let risultato = salva_configurazione_a_con_writer(
            &Configurazione::default(),
            &destinazione,
            |file, bytes| {
                use std::io::Write;
                file.write_all(&bytes[..bytes.len().min(8)])?;
                Err(std::io::Error::other("errore simulato dopo scrittura parziale"))
            },
        );

        assert!(risultato.is_err());
        assert_eq!(fs::read_to_string(&destinazione).unwrap(), "config precedente integra");
        let _ = fs::remove_dir_all(&directory);
    }

    #[test]
    fn salvataggio_atomico_sostituisce_configurazione_esistente() {
        let directory = std::env::temp_dir().join(format!(
            "focusanchor-config-atomic-success-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&directory);
        fs::create_dir(&directory).unwrap();
        let destinazione = directory.join("config.json");
        fs::write(&destinazione, "config precedente").unwrap();

        salva_configurazione_a(&Configurazione::default(), &destinazione).unwrap();

        let testo = fs::read_to_string(&destinazione).unwrap();
        assert!(serde_json::from_str::<Configurazione>(&testo).is_ok());
        let _ = fs::remove_dir_all(&directory);
    }

    #[test]
    fn ignora_le_righe_vuote_nelle_liste() {
        let lista = righe_alla_lista("reddit.com\n\n  youtube.com  \n");
        assert_eq!(lista, vec!["reddit.com", "youtube.com"]);
    }
}
