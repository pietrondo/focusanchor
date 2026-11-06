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
    /// Ultimo esame aperto.
    ///
    /// Serve a riaprire l'app con i materiali gia' davanti, invece di
    /// ricominciare dalla scelta dell'esame ogni volta.
    #[serde(default)]
    pub esame_corrente: String,
    /// Vero se la pagina di NotebookLM era aperta dentro la finestra.
    ///
    /// Riaprire l'app e ritrovare il notebook e' il punto dell'interfaccia
    /// unica; il profilo del browser conserva il login.
    #[serde(default)]
    pub notebooklm_incorporato: bool,
    /// Vero se l'intestazione e' ridotta a una riga.
    ///
    /// Con Zotero e NotebookLM affiancati lo spazio verticale e' prezioso:
    /// l'intestazione parte compressa e si allarga solo se serve.
    #[serde(default = "vero")]
    pub intestazione_compatta: bool,
    /// Quanto spazio prende la colonna di Zotero, da 0.2 a 0.8.
    ///
    /// Si trascina la linea in mezzo: e' il modo per dare piu' spazio alla
    /// pagina web o al PDF senza cambiare finestra.
    #[serde(default = "divisione_default")]
    pub divisione: f32,
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
            parole_vietate: default_parole(),
            cartella_zotero: String::new(),
            notebook_esami: std::collections::BTreeMap::new(),
            esami: std::collections::BTreeMap::new(),
            xp_punti: 0,
            xp_distrazioni: 0,
            modello_ai: modello_ai_default(),
            esame_corrente: String::new(),
            notebooklm_incorporato: false,
            intestazione_compatta: true,
            divisione: divisione_default(),
        }
    }
}

/// Divisione iniziale fra le due colonne: meta' e meta'.
fn divisione_default() -> f32 {
    0.52
}

/// Valore di default `true`, usato da serde quando il campo manca.
fn vero() -> bool {
    true
}

/// Modello AI predefinito: quello gia' configurato in OpenCode.
fn modello_ai_default() -> String {
    "opencode-go/deepseek-v4.1-flash".to_string()
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

/// Cartella dell'eseguibile.
///
/// Non e' piu' la cartella dei dati: li' l'app scriveva tutto e, se il
/// programma era installato in un percorso protetto, ogni scrittura
/// falliva in silenzio. Resta per i file che stanno accanto al programma.
pub fn cartella_programma() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.to_path_buf()))
        .unwrap_or_else(|| PathBuf::from("."))
}

/// Cartella dei dati dell'utente.
///
/// Dentro `%LOCALAPPDATA%\FocusAnchor`: li' si puo' sempre scrivere, e i
/// dati non spariscono quando il programma viene spostato o reinstallato.
pub fn cartella_dati() -> PathBuf {
    // Nei test i dati vanno in una cartella temporanea: la suite scrive
    // config e storico, e non deve toccare quelli veri dell'utente.
    #[cfg(test)]
    {
        use std::sync::Once;
        static PULIZIA: Once = Once::new();
        let cartella = std::env::temp_dir().join(format!(
            "focusanchor-test-{}",
            std::process::id()
        ));
        // Al primo uso si riparte da zero: le prove non devono vedersi
        // i file lasciati da un'esecuzione precedente.
        PULIZIA.call_once(|| {
            let _ = fs::remove_dir_all(&cartella);
        });
        let _ = fs::create_dir_all(&cartella);
        return cartella;
    }
    #[cfg(not(test))]
    std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .map(|base| base.join("FocusAnchor"))
        .unwrap_or_else(cartella_programma)
}

/// File che le versioni vecchie scrivevano accanto all'eseguibile.
const FILE_PROPRI: &[&str] = &[
    "config.json",
    "storico.json",
    "parcheggio_pensieri.txt",
    "riassunto_studio.md",
];

/// Prefissi dei file per esame: carte di ripasso, carte Anki, schede NotebookLM.
const PREFISSI_PROPRI: &[&str] = &["esame_", "anki_", "notebooklm_"];

/// Vero se il file e' uno di quelli che l'app salva.
///
/// Gli altri file della cartella (snapshot del database, profilo del
/// browser, temporanei) non sono dati dell'utente.
pub fn e_dato_dell_app(nome: &str) -> bool {
    FILE_PROPRI.contains(&nome) || PREFISSI_PROPRI.iter().any(|p| nome.starts_with(p))
}

/// Porta nella cartella dati i file delle versioni vecchie.
///
/// Va fatto una volta sola: senza, chi ha gia' studiato si ritroverebbe
/// con XP, storico e carte di ripasso a zero.
pub fn migra_dati_vecchi() {
    let vecchia = cartella_programma();
    let nuova = cartella_dati();
    if vecchia == nuova || !vecchia.is_dir() {
        return;
    }
    let Ok(voci) = fs::read_dir(&vecchia) else {
        return;
    };
    if fs::create_dir_all(&nuova).is_err() {
        return;
    }
    for voce in voci.flatten() {
        let nome = voce.file_name().to_string_lossy().into_owned();
        if !e_dato_dell_app(&nome) || !voce.file_type().map(|t| t.is_file()).unwrap_or(false) {
            continue;
        }
        let destinazione = nuova.join(&nome);
        if !destinazione.exists() {
            let _ = fs::copy(voce.path(), &destinazione);
        }
    }
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
    scrivi_atomico_con(percorso, &json, scrivi)
}

/// Scrive un file in modo atomico: prima su un temporaneo, poi rinomina.
///
/// Il rin finale e' atomico: il file precedente resta intatto se la
/// scrittura va storta, e il lettore non vede mai meta' file.
pub fn scrivi_atomico(percorso: &std::path::Path, contenuto: &[u8]) -> Result<(), String> {
    scrivi_atomico_con(percorso, contenuto, |file, bytes| file.write_all(bytes))
}

/// Come `scrivi_atomico`, ma con la scrittura iniettabile (per i test).
fn scrivi_atomico_con(
    percorso: &std::path::Path,
    contenuto: &[u8],
    scrivi: impl FnOnce(&mut File, &[u8]) -> io::Result<()>,
) -> Result<(), String> {
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
        scrivi(&mut file, contenuto)
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
