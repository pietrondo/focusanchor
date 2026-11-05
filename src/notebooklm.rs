//! Integrazione con NotebookLM tramite `notebooklm-py`.
//!
//! Il flusso di studio e': apri il notebook dell'esame (dove hai gia'
//! caricato i testi), chiedi quello che non ti e' chiaro, e la spiegazione
//! diventa materiale di ripasso.
//!
//! L'app parla con NotebookLM attraverso la CLI `notebooklm` installata con
//! `pip install notebooklm-py`: `ask - --json` legge la domanda da stdin e
//! restituisce la risposta in JSON. Niente chiavi nel programma, niente
//! dipendenze HTTP: si usa l'accesso Google che hai gia'.
//!
//! La libreria e' non ufficiale: se Google cambia le API interne si rompe.
//! Per questo resta anche il ripiego a mano (`prepara_domanda` + incolla),
//! che funziona finche' funziona il sito.

use std::io::Write;
use std::os::windows::process::CommandExt;
use std::process::{Command, Stdio};
use std::sync::mpsc::{self, Receiver, Sender};

use crate::blocker::CREATE_NO_WINDOW;

/// Nome dell'eseguibile installato da `pip install notebooklm-py`.
const ESEGUIBILE: &str = "notebooklm";

/// Secondi massimo di attesa per una risposta.
///
/// Oltre, meglio un errore chiaro che un'attesa senza fine: la risposta di
/// NotebookLM arriva di solito in pochi secondi.
const ATTESA_MASSIMA: u32 = 90;

/// Esito di una domanda a NotebookLM.
#[derive(Debug, Clone)]
pub enum Esito {
    /// NotebookLM ha risposto.
    Risposta(String),
    /// Qualcosa e' andato storto.
    Errore(String),
}

/// Una domanda in corso, con la sua risposta.
///
/// La chiamata sta in un thread: NotebookLM puo' metterci dieci secondi e
/// l'interfaccia non deve congelarsi.
pub struct Chiedi {
    canale: Receiver<Esito>,
    mittente: Sender<Esito>,
    /// Risposta arrivata, se c'e'.
    pub risposta: Option<String>,
    /// Vero mentre NotebookLM sta pensando.
    pub in_corso: bool,
    /// Esame a cui si riferisce la domanda in corso.
    pub esame: String,
}

impl Chiedi {
    /// Crea lo stato, senza nessuna richiesta in corso.
    pub fn nuovo() -> Self {
        let (mittente, canale) = mpsc::channel();
        Self {
            canale,
            mittente,
            risposta: None,
            in_corso: false,
            esame: String::new(),
        }
    }

    /// Manda una domanda al notebook dell'esame.
    ///
    /// Se c'e' gia' una richiesta in corso non ne parte un'altra: due
    /// processi insieme consumano quota per niente.
    pub fn chiedi(&mut self, notebook: &str, esame: &str, domanda: &str) {
        if self.in_corso {
            return;
        }
        let domanda = domanda.trim();
        if domanda.is_empty() || notebook.trim().is_empty() {
            return;
        }
        self.risposta = None;
        self.in_corso = true;
        self.esame = esame.to_string();

        let mittente = self.mittente.clone();
        let notebook = notebook.to_string();
        let domanda = domanda.to_string();
        std::thread::spawn(move || {
            let esito = esegui(&notebook, &domanda);
            let _ = mittente.send(esito);
        });
    }

    /// Ritira l'esito, se e' arrivato.
    pub fn aggiorna(&mut self) {
        while let Ok(esito) = self.canale.try_recv() {
            self.in_corso = false;
            match esito {
                Esito::Risposta(testo) => self.risposta = Some(testo),
                Esito::Errore(messaggio) => self.risposta = Some(messaggio),
            }
        }
    }

    /// Cancella la risposta mostrata.
    pub fn pulisci(&mut self) {
        self.risposta = None;
        self.esame.clear();
    }
}

/// Un notebook come lo elenca la CLI.
#[derive(Debug, Clone)]
pub struct Notebook {
    /// Identificativo del notebook.
    pub id: String,
    /// Titolo mostrato in NotebookLM.
    pub titolo: String,
}

/// Una fonte caricata in un notebook.
#[derive(Debug, Clone)]
pub struct Fonte {
    /// Titolo della fonte.
    pub titolo: String,
    /// Tipo (PDF, testo, chat, ...).
    pub tipo: String,
    /// Vero se NotebookLM ha finito di elaborarla.
    pub pronta: bool,
}

/// Una flashcard generata da NotebookLM.
#[derive(Debug, Clone)]
pub struct Flashcard {
    /// La domanda.
    pub domanda: String,
    /// La risposta.
    pub risposta: String,
}

/// Lavoro lungo in corso: generazione di flashcard o caricamento di fonti.
///
/// Come per le domande, il processo sta in un thread: generare un mazzo
/// puo' richiedere minuti e l'interfaccia non deve congelarsi.
pub struct Genera {
    canale: Receiver<Generazione>,
    mittente: Sender<Generazione>,
    /// Esito arrivato, se c'e'.
    pub esito: Option<Generazione>,
    /// Vero mentre NotebookLM sta lavorando.
    pub in_corso: bool,
    /// Esame a cui si riferisce il lavoro.
    pub esame: String,
}

/// Esito di un lavoro lungo.
#[derive(Debug, Clone)]
pub enum Generazione {
    /// Flashcard pronte da importare.
    Flashcard(Vec<Flashcard>),
    /// Fonti caricate o elenco aggiornato.
    Fonti(Vec<Fonte>),
    /// Lavoro concluso senza dati da mostrare.
    Fatto(String),
    /// Qualcosa e' andato storto.
    Errore(String),
}

impl Genera {
    /// Crea lo stato, senza nessun lavoro in corso.
    pub fn nuovo() -> Self {
        let (mittente, canale) = mpsc::channel();
        Self {
            canale,
            mittente,
            esito: None,
            in_corso: false,
            esame: String::new(),
        }
    }

    /// Genera le flashcard del notebook e le restituisce.
    pub fn flashcard(&mut self, notebook: &str, esame: &str) {
        if self.in_corso {
            return;
        }
        self.esito = None;
        self.in_corso = true;
        self.esame = esame.to_string();

        let mittente = self.mittente.clone();
        let notebook = notebook.to_string();
        std::thread::spawn(move || {
            let esito = match genera_flashcard(&notebook) {
                Ok(carte) => Generazione::Flashcard(carte),
                Err(e) => Generazione::Errore(e),
            };
            let _ = mittente.send(esito);
        });
    }

    /// Legge le fonti del notebook.
    pub fn fonti(&mut self, notebook: &str, esame: &str) {
        if self.in_corso {
            return;
        }
        self.esito = None;
        self.in_corso = true;
        self.esame = esame.to_string();

        let mittente = self.mittente.clone();
        let notebook = notebook.to_string();
        std::thread::spawn(move || {
            let esito = match elenco_fonti(&notebook) {
                Ok(fonti) => Generazione::Fonti(fonti),
                Err(e) => Generazione::Errore(e),
            };
            let _ = mittente.send(esito);
        });
    }

    /// Carica un file locale come fonte del notebook.
    pub fn carica(&mut self, notebook: &str, esame: &str, percorso: std::path::PathBuf) {
        if self.in_corso {
            return;
        }
        self.esito = None;
        self.in_corso = true;
        self.esame = esame.to_string();

        let mittente = self.mittente.clone();
        let notebook = notebook.to_string();
        std::thread::spawn(move || {
            let esito = match aggiungi_fonte(&notebook, &percorso) {
                Ok(()) => Generazione::Fatto(format!(
                    "{} caricata come fonte nel notebook.",
                    percorso
                        .file_name()
                        .map(|n| n.to_string_lossy().into_owned())
                        .unwrap_or_default()
                )),
                Err(e) => Generazione::Errore(e),
            };
            let _ = mittente.send(esito);
        });
    }

    /// Ritira l'esito, se e' arrivato.
    pub fn aggiorna(&mut self) {
        while let Ok(esito) = self.canale.try_recv() {
            self.in_corso = false;
            self.esito = Some(esito);
        }
    }

    /// Dimentica l'esito mostrato.
    pub fn pulisci(&mut self) {
        self.esito = None;
        self.esame.clear();
    }
}

/// Genera un mazzo di flashcard dalle fonti del notebook.
///
/// Usa `--wait`: aspetta che NotebookLM abbia finito invece di lasciare un
/// lavoro appeso che nessuno guarda piu'.
fn genera_flashcard(notebook: &str) -> Result<Vec<Flashcard>, String> {
    let uscita = Command::new(ESEGUIBILE)
        .args([
            "generate",
            "flashcards",
            "-n",
            notebook,
            "--language",
            "it",
            "--wait",
            "--timeout",
            "300",
            "--json",
        ])
        .creation_flags(CREATE_NO_WINDOW)
        .stdin(Stdio::null())
        .output()
        .map_err(|e| avvio_fallito(&e))?;
    if !uscita.status.success() {
        let stderr = String::from_utf8_lossy(&uscita.stderr);
        return Err(messaggio_errore("", &stderr));
    }

    // Il mazzo si scarica dall'artefatto piu' recente.
    let percorso = std::env::temp_dir().join(format!(
        "focusanchor-flashcard-{}-{}.json",
        std::process::id(),
        notebook.chars().take(8).collect::<String>()
    ));
    let scarica = Command::new(ESEGUIBILE)
        .args([
            "download",
            "flashcards",
            &percorso.to_string_lossy(),
            "-n",
            notebook,
        ])
        .creation_flags(CREATE_NO_WINDOW)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .output()
        .map_err(|e| avvio_fallito(&e))?;
    let testo = std::fs::read_to_string(&percorso).map_err(|e| {
        format!(
            "NotebookLM non ha prodotto il file delle flashcard: {e} {}",
            prima_riga(&String::from_utf8_lossy(&scarica.stderr))
        )
    });
    let _ = std::fs::remove_file(&percorso);
    leggi_flashcard(&testo?)
}

/// Estrae le carte dal JSON di `download flashcards`.
fn leggi_flashcard(testo: &str) -> Result<Vec<Flashcard>, String> {
    let valore: serde_json::Value = serde_json::from_str(testo.trim())
        .map_err(|e| format!("Flashcard non leggibili: {e}"))?;
    let mazzo = valore
        .get("cards")
        .and_then(|c| c.as_array())
        .ok_or_else(|| "Il mazzo non contiene carte.".to_string())?;
    Ok(mazzo
        .iter()
        .filter_map(|carta| {
            let domanda = testo_blocchi(carta.get("front"));
            let risposta = testo_blocchi(carta.get("back"));
            if domanda.is_empty() || risposta.is_empty() {
                return None;
            }
            Some(Flashcard { domanda, risposta })
        })
        .collect())
}

/// Unisce i blocchi di testo di un lato della carta.
fn testo_blocchi(lato: Option<&serde_json::Value>) -> String {
    lato.and_then(|l| l.get("flashcardContentBlock"))
        .and_then(|b| b.as_array())
        .map(|blocchi| {
            blocchi
                .iter()
                .filter_map(|b| b.get("content").and_then(|c| c.as_str()))
                .map(|c| c.trim())
                .filter(|c| !c.is_empty())
                .collect::<Vec<_>>()
                .join(" ")
        })
        .unwrap_or_default()
}

/// Elenca le fonti caricate in un notebook.
pub fn elenco_fonti(notebook: &str) -> Result<Vec<Fonte>, String> {
    let uscita = Command::new(ESEGUIBILE)
        .args(["source", "list", "-n", notebook, "--json"])
        .creation_flags(CREATE_NO_WINDOW)
        .stdin(Stdio::null())
        .output()
        .map_err(|e| avvio_fallito(&e))?;
    if !uscita.status.success() {
        let stderr = String::from_utf8_lossy(&uscita.stderr);
        return Err(messaggio_errore("", &stderr));
    }
    leggi_fonti(&String::from_utf8_lossy(&uscita.stdout))
}

/// Legge il JSON di `source list`.
fn leggi_fonti(testo: &str) -> Result<Vec<Fonte>, String> {
    let valore: serde_json::Value = serde_json::from_str(testo.trim())
        .map_err(|e| format!("Elenco delle fonti non leggibile: {e}"))?;
    let fonti = valore
        .get("sources")
        .and_then(|s| s.as_array())
        .ok_or_else(|| "NotebookLM non ha restituito le fonti.".to_string())?;
    Ok(fonti
        .iter()
        .map(|f| Fonte {
            titolo: f
                .get("title")
                .and_then(|t| t.as_str())
                .unwrap_or("senza titolo")
                .to_string(),
            tipo: f
                .get("type")
                .and_then(|t| t.as_str())
                .unwrap_or("")
                .to_string(),
            pronta: f.get("status").and_then(|s| s.as_str()) == Some("ready"),
        })
        .collect())
}

/// Carica un file come fonte del notebook.
///
/// E' il ponte che mancava: la scheda generata dall'app si caricava su
/// NotebookLM trascinandola a mano nel browser.
pub fn aggiungi_fonte(notebook: &str, percorso: &std::path::Path) -> Result<(), String> {
    if !percorso.exists() {
        return Err(format!("{} non esiste ancora.", percorso.display()));
    }
    let uscita = Command::new(ESEGUIBILE)
        .args([
            "source",
            "add",
            &percorso.to_string_lossy(),
            "-n",
            notebook,
            "--type",
            "file",
            "--request-timeout",
            "120",
        ])
        .creation_flags(CREATE_NO_WINDOW)
        .stdin(Stdio::null())
        .output()
        .map_err(|e| avvio_fallito(&e))?;
    if uscita.status.success() {
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&uscita.stderr);
        Err(messaggio_errore("", &stderr))
    }
}

/// Messaggio comune quando la CLI non parte.
fn avvio_fallito(errore: &std::io::Error) -> String {
    format!(
        "Non trovo la CLI `notebooklm`: {errore}. Installa con \
         `pip install notebooklm-py` e accedi con `notebooklm login`."
    )
}


/// Chiede a NotebookLM e restituisce la risposta.
fn esegui(notebook: &str, domanda: &str) -> Esito {
    let mut figlio = match Command::new(ESEGUIBILE)
        .args([
            "ask",
            "-",
            "-n",
            notebook,
            "--json",
            "--request-timeout",
            &ATTESA_MASSIMA.to_string(),
        ])
        .creation_flags(CREATE_NO_WINDOW)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
    {
        Ok(figlio) => figlio,
        Err(e) => {
            return Esito::Errore(format!(
                "Non trovo la CLI `notebooklm`: {e}. Installa con \
                 `pip install notebooklm-py` e accedi con `notebooklm login`."
            ))
        }
    };

    // La domanda passa da stdin: niente problemi di quoting con virgolette,
    // a capo e caratteri accentati.
    if let Some(mut stdin) = figlio.stdin.take() {
        let _ = stdin.write_all(domanda.as_bytes());
    }

    let uscita = match figlio.wait_with_output() {
        Ok(uscita) => uscita,
        Err(e) => return Esito::Errore(format!("NotebookLM non ha risposto: {e}")),
    };

    let stdout = String::from_utf8_lossy(&uscita.stdout);
    if let Some(risposta) = estrai_risposta(&stdout) {
        return Esito::Risposta(pulisci_markdown(&risposta));
    }

    // Senza JSON valido si guarda l'errore: il caso piu' comune e' la
    // sessione scaduta, e va detto con le parole giuste.
    let stderr = String::from_utf8_lossy(&uscita.stderr);
    Esito::Errore(messaggio_errore(&stdout, &stderr))
}

/// Estrae il campo `answer` dal JSON della CLI.
fn estrai_risposta(stdout: &str) -> Option<String> {
    let valore: serde_json::Value = serde_json::from_str(stdout.trim()).ok()?;
    let risposta = valore.get("answer")?.as_str()?.trim().to_string();
    if risposta.is_empty() {
        None
    } else {
        Some(risposta)
    }
}

/// Costruisce un messaggio d'errore leggibile.
fn messaggio_errore(stdout: &str, stderr: &str) -> String {
    let testo = format!("{stderr}\n{stdout}").to_lowercase();
    if testo.contains("storage") || testo.contains("login") || testo.contains("auth") {
        return "NotebookLM non e' autenticato: esegui `notebooklm login` una volta, \
                poi riprova."
            .into();
    }
    if testo.contains("notebook") && testo.contains("not found") {
        return "Il notebook di questo esame non esiste piu': ricollega il link.".into();
    }
    let dettaglio = stderr.trim();
    if dettaglio.is_empty() {
        format!("NotebookLM non ha risposto. {}", prima_riga(stdout))
    } else {
        format!("NotebookLM: {}", prima_riga(dettaglio))
    }
}

/// Prima riga utile di un testo, accorciata.
///
/// Un traceback comincia con "Traceback (most recent call last)": mostrare
/// quella riga non dice niente. Si preferisce la riga che contiene
/// l'errore vero.
fn prima_riga(testo: &str) -> String {
    let righe: Vec<&str> = testo
        .lines()
        .map(|riga| riga.trim())
        .filter(|riga| !riga.is_empty())
        .collect();
    let scelta = righe
        .iter()
        .find(|riga| {
            let bassa = riga.to_lowercase();
            bassa.contains("error") || bassa.contains("errore")
        })
        .or_else(|| righe.first());
    scelta
        .map(|riga| riga.chars().take(200).collect())
        .unwrap_or_else(|| "errore sconosciuto".to_string())
}

/// Toglie i marcatori Markdown che l'app non sa disegnare.
///
/// NotebookLM risponde in grassetto (`**testo**`): nel pannello e nelle
/// carte gli asterischi restano a vista, quindi si tolgono.
fn pulisci_markdown(testo: &str) -> String {
    testo.replace("**", "").replace("__", "")
}

/// Elenca i notebook dell'account.
///
/// Serve a collegare gli esami ai loro notebook senza che l'utente debba
/// copiare cinque link a mano.
pub fn elenco_notebook() -> Result<Vec<Notebook>, String> {
    let uscita = Command::new(ESEGUIBILE)
        .args(["list", "--json"])
        .creation_flags(CREATE_NO_WINDOW)
        .stdin(Stdio::null())
        .output()
        .map_err(|e| {
            format!(
                "Non trovo la CLI `notebooklm`: {e}. Installa con \
                 `pip install notebooklm-py`."
            )
        })?;
    if !uscita.status.success() {
        let stderr = String::from_utf8_lossy(&uscita.stderr);
        return Err(messaggio_errore("", &stderr));
    }
    let testo = String::from_utf8_lossy(&uscita.stdout);
    leggi_elenco(&testo)
}

/// Legge il JSON di `notebooklm list --json`.
fn leggi_elenco(testo: &str) -> Result<Vec<Notebook>, String> {
    let valore: serde_json::Value = serde_json::from_str(testo.trim())
        .map_err(|e| format!("Risposta di NotebookLM non leggibile: {e}"))?;
    let elenco = valore
        .get("notebooks")
        .and_then(|n| n.as_array())
        .ok_or_else(|| "NotebookLM non ha restituito nessun elenco.".to_string())?;
    Ok(elenco
        .iter()
        .filter_map(|n| {
            Some(Notebook {
                id: n.get("id")?.as_str()?.to_string(),
                titolo: n.get("title").and_then(|t| t.as_str()).unwrap_or("").to_string(),
            })
        })
        .collect())
}

/// Estrae l'identificativo del notebook da un link.
pub fn id_da_link(link: &str) -> Option<String> {
    let dopo = link.split("/notebook/").nth(1)?;
    let id = dopo.split(['?', '#', '/']).next()?.trim();
    if id.is_empty() {
        None
    } else {
        Some(id.to_string())
    }
}

/// Costruisce il link del notebook dal suo identificativo.
pub fn link_da_id(id: &str) -> String {
    format!("https://notebooklm.google.com/notebook/{id}")
}

/// Normalizza un nome per confrontare esami e titoli dei notebook.
///
/// Gli esami vengono da Zotero ("storia romana"), i notebook da NotebookLM
/// ("Storia Romana"): per confrontarli servono minuscole e sole lettere.
fn normalizza(nome: &str) -> String {
    nome.chars()
        .filter(|c| c.is_alphanumeric())
        .flat_map(|c| c.to_lowercase())
        .collect()
}

/// Trova il notebook che corrisponde a un esame.
///
/// Il confronto e' permissivo: l'esame puo' avere un suffisso che il titolo
/// del notebook non ha (o viceversa), quindi basta che uno contenga
/// l'altro. Fra due corrispondenze vince la piu' corta, che e' quella
/// meno ambigua.
pub fn abbina(esame: &str, notebook: &[Notebook]) -> Option<String> {
    let cercato = normalizza(esame);
    if cercato.is_empty() {
        return None;
    }
    notebook
        .iter()
        .filter(|n| {
            let titolo = normalizza(&n.titolo);
            !titolo.is_empty() && (titolo == cercato || titolo.contains(&cercato) || cercato.contains(&titolo))
        })
        .min_by_key(|n| normalizza(&n.titolo).len())
        .map(|n| n.id.clone())
}


/// Lunghezza massima del testo riportato nel messaggio.
///
/// Oltre questa soglia il messaggio diventa illeggibile e NotebookLM non
/// guadagna nulla: meglio un passaggio mirato.
pub const MASSIMO_CARATTERI: usize = 4000;

/// Costruisce il messaggio pronto da incollare nella chat di NotebookLM.
///
/// Il messaggio dice all'esame di riferimento, riporta il passaggio copiato
/// e chiede una spiegazione piu' chiara: e' la stessa richiesta che l'utente
/// farebbe a mano, ma senza doverla scrivere ogni volta.
pub fn prepara_messaggio(esame: &str, passaggio: &str) -> String {
    let passaggio = passaggio.trim();
    if passaggio.is_empty() {
        return String::new();
    }
    let esame = if esame.trim().is_empty() {
        "questo esame"
    } else {
        esame.trim()
    };

    // Un passaggio troppo lungo viene tagliato a un confine di parola, per
    // non spezzare una frase a meta'.
    let estratto = if passaggio.chars().count() > MASSIMO_CARATTERI {
        let tagliato: String = passaggio.chars().take(MASSIMO_CARATTERI).collect();
        match tagliato.rfind(' ') {
            Some(pos) => format!("{} [...]", &tagliato[..pos]),
            None => format!("{tagliato} [...]"),
        }
    } else {
        passaggio.to_string()
    };

    format!(
        "Sto studiando per l'esame di {esame} e non mi e' chiaro questo passaggio \
         dei testi che ti ho caricato.\n\n\
         «{estratto}»\n\n\
         Me lo spieghi meglio, con parole semplici e un esempio concreto? \
         Se utile, collegalo agli altri testi dello stesso esame."
    )
}

/// Compone la domanda per un esame e una domanda libera.
///
/// Diverso da `prepara_messaggio`: qui e' l'utente a fare la domanda
/// ("fammi un riassunto", "quali sono i concetti chiave?") e l'app aggiunge
/// solo il contesto dell'esame.
pub fn prepara_domanda(esame: &str, domanda: &str) -> String {
    let domanda = domanda.trim();
    if domanda.is_empty() {
        return String::new();
    }
    let esame = if esame.trim().is_empty() {
        "questo esame"
    } else {
        esame.trim()
    };
    format!(
        "Sto preparando l'esame di {esame}. Rispondi in italiano, con parole semplici \
         e un esempio concreto, basandoti sui testi che ti ho caricato.\n\n\
         {domanda}"
    )
}

/// Copia il messaggio negli appunti, pronti da incollare.
pub fn copia_negli_appunti(testo: &str) -> Result<(), String> {
    let mut clipboard = arboard::Clipboard::new().map_err(|e| e.to_string())?;
    clipboard
        .set_text(testo.to_string())
        .map_err(|e| format!("Non riesco a copiare il messaggio: {e}"))
}

/// Apre un indirizzo nel browser predefinito.
pub fn apri(url: &str) -> Result<(), String> {
    // Solo http/https: un link incollato non deve poter lanciare altro.
    if !(url.starts_with("https://") || url.starts_with("http://")) {
        return Err("Il link del notebook deve iniziare con https://".into());
    }
    Command::new("cmd")
        .args(["/C", "start", "", url])
        .creation_flags(CREATE_NO_WINDOW)
        .spawn()
        .map(|_| ())
        .map_err(|e| format!("Non riesco ad aprire il browser: {e}"))
}

/// Normalizza il link incollato dall'utente.
///
/// Chi copia dalla barra degli indirizzi puo' incollare senza schema o con
/// parametri di tracking: si tiene solo l'essenziale.
pub fn normalizza_link(link: &str) -> Option<String> {
    let link = link.trim();
    if link.is_empty() {
        return None;
    }
    let completo = if link.starts_with("http://") || link.starts_with("https://") {
        link.to_string()
    } else if link.contains("notebooklm.google.com") {
        format!("https://{link}")
    } else {
        return None;
    };
    // Via i parametri di tracking: rendono il link fragile e illeggibile.
    let senza_parametri = completo
        .split('?')
        .next()
        .unwrap_or(&completo)
        .trim_end_matches('/')
        .to_string();
    Some(senza_parametri)
}

/// Riconosce un link di NotebookLM, anche scritto a mano.
///
/// Il link del notebook e' l'unica cosa che l'utente deve salvare a mano:
/// tutto il resto lo fa l'app. Riconoscerlo qui evita di chiedere un link
/// che e' gia' presente.
pub fn riconosci_link(testo: &str) -> Option<String> {
    let testo = testo.trim();
    // Il link puo' essere sparso in una frase: "e' qui https://notebooklm..."
    let dentro = testo
        .split_whitespace()
        .find(|parola| parola.contains("notebooklm.google.com"))?;
    normalizza_link(dentro)
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn la_risposta_si_estrae_dal_json_della_cli() {
        let stdout = r#"{
          "answer": "**Le fonti trattano della Roma arcaica.**",
          "conversation_id": "74888dba",
          "turn_number": 66,
          "references": []
        }"#;

        let risposta = estrai_risposta(stdout).expect("risposta");

        // I marcatori del grassetto non si disegnano nell'app: si tolgono.
        assert_eq!(pulisci_markdown(&risposta), "Le fonti trattano della Roma arcaica.");
    }

    #[test]
    fn senza_campo_answer_non_si_inventa_una_risposta() {
        assert!(estrai_risposta(r#"{"status":"ok"}"#).is_none());
        assert!(estrai_risposta("non e' json").is_none());
        assert!(estrai_risposta(r#"{"answer":"   "}"#).is_none());
    }

    #[test]
    fn una_sessione_scaduta_lo_dice_in_modo_comprensibile() {
        let messaggio = messaggio_errore("", "Error: Storage file not found");

        assert!(messaggio.contains("notebooklm login"));
    }

    #[test]
    fn l_errore_porta_la_riga_vera_non_un_muro_di_log() {
        let messaggio = messaggio_errore("", "Traceback...\nError: quota superata\nriga due");

        assert!(messaggio.contains("quota superata"));
        assert!(!messaggio.contains("riga due"));
    }

    #[test]
    fn l_identificativo_si_estrae_dal_link() {
        assert_eq!(
            id_da_link("https://notebooklm.google.com/notebook/ff3268d8-2213?authuser=0")
                .as_deref(),
            Some("ff3268d8-2213")
        );
        assert_eq!(
            id_da_link("https://notebooklm.google.com/notebook/ff3268d8/").as_deref(),
            Some("ff3268d8")
        );
        assert!(id_da_link("https://example.com/altro").is_none());
    }

    #[test]
    fn il_link_si_ricostruisce_dall_identificativo() {
        assert_eq!(
            link_da_id("abc123"),
            "https://notebooklm.google.com/notebook/abc123"
        );
    }

    #[test]
    fn un_esame_trova_il_suo_notebook_anche_con_nome_diverso() {
        let notebook = vec![
            Notebook {
                id: "uno".into(),
                titolo: "Storia Romana".into(),
            },
            Notebook {
                id: "due".into(),
                titolo: "Letteratura italiana".into(),
            },
        ];

        // Zotero scrive l'esame in minuscolo, NotebookLM in maiuscolo.
        assert_eq!(abbina("storia romana", &notebook).as_deref(), Some("uno"));
        assert_eq!(
            abbina("Letteratura italiana", &notebook).as_deref(),
            Some("due")
        );
    }

    #[test]
    fn un_esame_con_suffisso_trova_lo_stesso_il_notebook() {
        let notebook = vec![Notebook {
            id: "eco".into(),
            titolo: "Economia e storia della globalizzazione".into(),
        }];

        // In config l'esame ha anche la data nel nome.
        assert_eq!(
            abbina("Economia e storia della globalizzazione 19/10/2026, 22:53:52", &notebook)
                .as_deref(),
            Some("eco")
        );
    }

    #[test]
    fn fra_due_corrispondenze_vince_la_piu_precisa() {
        let notebook = vec![
            Notebook {
                id: "lungo".into(),
                titolo: "Storia del Diritto in Europa".into(),
            },
            Notebook {
                id: "corto".into(),
                titolo: "Storia del diritto".into(),
            },
        ];

        // "Storia del diritto" non deve finire sul notebook di un altro corso.
        assert_eq!(
            abbina("Storia del diritto", &notebook).as_deref(),
            Some("corto")
        );
    }

    #[test]
    fn un_esame_senza_notebook_non_ne_prende_uno_a_caso() {
        let notebook = vec![Notebook {
            id: "uno".into(),
            titolo: "Storia Romana".into(),
        }];

        assert!(abbina("Fisica nucleare", &notebook).is_none());
    }

    #[test]
    fn l_elenco_dei_notebook_si_legge() {
        let json = r#"{"notebooks":[
            {"id":"a1","title":"Storia Romana"},
            {"id":"b2","title":"Latino"}
        ]}"#;

        let elenco = leggi_elenco(json).expect("elenco");

        assert_eq!(elenco.len(), 2);
        assert_eq!(elenco[0].id, "a1");
        assert_eq!(elenco[1].titolo, "Latino");
    }

    #[test]
    fn le_flashcard_si_leggono_dal_mazzo() {
        let json = r#"{"title":"Mazzo","cards":[
            {"front":{"flashcardContentBlock":[{"type":"text","content":"Chi era Augusto?"}]},
             "back":{"flashcardContentBlock":[{"type":"text","content":"Il primo imperatore."}]}},
            {"front":{"flashcardContentBlock":[{"type":"text","content":"  "}]},
             "back":{"flashcardContentBlock":[{"type":"text","content":"vuota"}]}}
        ]}"#;

        let carte = leggi_flashcard(json).expect("carte");

        // La carta senza domanda non entra: nel ripasso non interrogherebbe.
        assert_eq!(carte.len(), 1);
        assert_eq!(carte[0].domanda, "Chi era Augusto?");
        assert_eq!(carte[0].risposta, "Il primo imperatore.");
    }

    #[test]
    fn le_fonti_si_leggono_con_il_loro_stato() {
        let json = r#"{"sources":[
            {"title":"Manuale","type":"pdf","status":"ready"},
            {"title":"In corso","type":"web_page","status":"preparing"}
        ]}"#;

        let fonti = leggi_fonti(json).expect("fonti");

        assert_eq!(fonti.len(), 2);
        assert!(fonti[0].pronta);
        assert_eq!(fonti[0].tipo, "pdf");
        assert!(!fonti[1].pronta);
    }

    #[test]
    fn il_messaggio_contiene_esame_e_passaggio() {
        let m = prepara_messaggio("Storia Romana", "La riforma di Tiberio Gracco");
        assert!(m.contains("Storia Romana"));
        assert!(m.contains("La riforma di Tiberio Gracco"));
        assert!(m.contains("spieghi meglio"));
    }

    #[test]
    fn un_passaggio_vuoto_non_produce_messaggio() {
        assert!(prepara_messaggio("Storia", "   ").is_empty());
    }

    #[test]
    fn un_passaggio_lungo_viene_tagliato_a_confine_di_parola() {
        let lungo = "parola ".repeat(MASSIMO_CARATTERI);
        let m = prepara_messaggio("Storia", &lungo);
        // Il messaggio resta leggibile e segnala il taglio.
        assert!(m.contains("[...]"));
        assert!(m.chars().count() < lungo.chars().count());
    }

    #[test]
    fn senza_esame_il_messaggio_resta_comprensibile() {
        let m = prepara_messaggio("", "un passaggio");
        assert!(m.contains("questo esame"));
        assert!(m.contains("un passaggio"));
    }

    #[test]
    fn il_link_viene_normalizzato() {
        // Con parametri di tracking: si tagliano.
        assert_eq!(
            normalizza_link("https://notebooklm.google.com/notebook/abc123?authuser=0").unwrap(),
            "https://notebooklm.google.com/notebook/abc123"
        );
        // Senza schema ma con il dominio giusto: si aggiunge https.
        assert_eq!(
            normalizza_link("notebooklm.google.com/notebook/abc123").unwrap(),
            "https://notebooklm.google.com/notebook/abc123"
        );
        // Roba che non e' un link: rifiutata.
        assert!(normalizza_link("ciao").is_none());
        assert!(normalizza_link("   ").is_none());
    }

    #[test]
    fn un_link_sparso_in_una_frase_viene_riconosciuto() {
        let testo = "guarda qui https://notebooklm.google.com/notebook/xyz789 ?authuser=0";
        assert_eq!(
            riconosci_link(testo).as_deref(),
            Some("https://notebooklm.google.com/notebook/xyz789")
        );
    }

    #[test]
    fn una_frase_senza_link_non_inventa_nulla() {
        assert!(riconosci_link("non c'e' nessun link qui").is_none());
    }

    #[test]
    fn la_domanda_libera_aggiunge_il_contesto_dell_esame() {
        let m = prepara_domanda("Storia romana", "Fammi un riassunto del capitolo 3");
        assert!(m.contains("Storia romana"));
        assert!(m.contains("Fammi un riassunto del capitolo 3"));
        assert!(m.contains("italiano"));
        assert!(prepara_domanda("Storia", "  ").is_empty());
    }
}
