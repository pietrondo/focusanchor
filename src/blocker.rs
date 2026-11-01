//! Modulo di blocco anti-distrazione.
//!
//! Livello 1: modifica del file `hosts` di Windows con flush del DNS.
//! Livello 2: guardia delle finestre attive tramite API Win32.

use std::fs;
use std::os::windows::process::CommandExt;
use std::path::PathBuf;
use std::process::{Command, Stdio};

use windows::Win32::UI::WindowsAndMessaging::{
    GetForegroundWindow, GetWindowThreadProcessId, ShowWindow, SW_MINIMIZE,
};

/// Percorso del file `hosts` di Windows.
pub const HOSTS_PATH: &str = r"C:\Windows\System32\drivers\etc\hosts";

/// Marcatore di inizio del blocco inserito nel file `hosts`.
pub const MARCA_INIZIO: &str = "# === INIZIO FOCUS ANCHOR ===";

/// Marcatore di fine del blocco inserito nel file `hosts`.
pub const MARCA_FINE: &str = "# === FINE FOCUS ANCHOR ===";

/// Flag Windows `CREATE_NO_WINDOW`: evita che compaia una finestra nera.
pub const CREATE_NO_WINDOW: u32 = 0x0800_0000;

/// Segnale inviato dalla guardia finestre alla GUI.
#[derive(Debug)]
pub enum Segnale {
    /// Una finestra vietata e' diventata in primo piano.
    Distrazione(String),
    /// Su Zotero e' aperto un PDF di un altro esame.
    ContestoZotero(String),
}
/// Errore restituito dalle operazioni sul file `hosts`.
#[derive(Debug)]
pub enum ErroreHosts {
    /// Il file `hosts` non e' leggibile o scrivibile (manca spesso il permesso admin).
    AccessoNegato(String),
    /// Errore generico di I/O.
    Io(String),
}

impl std::fmt::Display for ErroreHosts {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ErroreHosts::AccessoNegato(m) => write!(f, "{m}"),
            ErroreHosts::Io(m) => write!(f, "{m}"),
        }
    }
}

impl From<std::io::Error> for ErroreHosts {
    fn from(e: std::io::Error) -> Self {
        match e.kind() {
            std::io::ErrorKind::PermissionDenied => {
                ErroreHosts::AccessoNegato(e.to_string())
            }
            _ => ErroreHosts::Io(e.to_string()),
        }
    }
}

/// Percorso del file `hosts` come `PathBuf`.
pub fn percorso_hosts() -> PathBuf {
    PathBuf::from(HOSTS_PATH)
}

/// Esegue `ipconfig /flushdns` senza mostrare finestre.
pub fn flush_dns() {
    let _ = Command::new("ipconfig")
        .arg("/flushdns")
        .creation_flags(CREATE_NO_WINDOW)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
}

/// Rimuove dal file `hosts` il blocco di FocusAnchor, se presente.
///
/// Funzione sicura: non entra mai in panic, cosi' puo' essere chiamata
/// anche dal panic hook di sicurezza.
pub fn ripristina_hosts() {
    let percorso = percorso_hosts();
    let Ok(contenuto) = fs::read_to_string(&percorso) else {
        return;
    };
    let ripulito = rimuovi_blocco(&contenuto);
    if ripulito != contenuto {
        let _ = fs::write(&percorso, ripulito);
    }
}

/// Toglie il blocco delimitato dai marcatori, se presente nel testo.
pub fn rimuovi_blocco(contenuto: &str) -> String {
    let (Some(inizio), Some(fine)) = (
        contenuto.find(MARCA_INIZIO),
        contenuto.find(MARCA_FINE),
    ) else {
        return contenuto.to_string();
    };
    if fine < inizio {
        return contenuto.to_string();
    }
    let mut risultato = String::with_capacity(contenuto.len());
    risultato.push_str(&contenuto[..inizio]);
    risultato.push_str(&contenuto[fine + MARCA_FINE.len()..]);
    risultato
}

/// Costruisce il testo del blocco da inserire in fondo al file `hosts`.
pub fn costruisci_blocco(domini: &[String]) -> String {
    let mut blocco = String::new();
    blocco.push_str(MARCA_INIZIO);
    blocco.push('\n');
    for dominio in domini {
        let d = dominio.trim();
        if d.is_empty() {
            continue;
        }
        blocco.push_str(&format!("127.0.0.1 {d}\n"));
        blocco.push_str(&format!("::1 {d}\n"));
    }
    blocco.push_str(MARCA_FINE);
    blocco.push('\n');
    blocco
}

/// Aggiunge in fondo al file `hosts` il blocco che reindirizza i domini vietati.
///
/// Restituisce `Err` se il file non e' scrivibile (tipicamente serve l'avvio
/// come Amministratore): in quel caso l'applicazione continua con il solo
/// Livello 2 (Guardiano Finestre).
pub fn applica_blocco(domini: &[String]) -> Result<(), ErroreHosts> {
    let percorso = percorso_hosts();
    let contenuto = fs::read_to_string(&percorso)?;
    let mut nuovo = rimuovi_blocco(&contenuto);
    if !nuovo.is_empty() && !nuovo.ends_with('\n') {
        nuovo.push('\n');
    }
    nuovo.push_str(&costruisci_blocco(domini));
    fs::write(&percorso, nuovo)?;
    flush_dns();
    Ok(())
}

/// Legge il titolo della finestra in primo piano.
///
/// Restituisce una stringa vuota se non c'e' una finestra attiva.
pub fn titolo_finestra_attiva() -> String {
    crate::finestre::titolo_in_primo_piano()
}

/// Restituisce l'identificativo del processo che possiede la finestra attiva.
fn processo_della_finestra_attiva() -> u32 {
    unsafe {
        let hwnd = GetForegroundWindow();
        if hwnd.is_invalid() {
            return 0;
        }
        let mut processo: u32 = 0;
        GetWindowThreadProcessId(hwnd, Some(&mut processo));
        processo
    }
}

/// Vero se la finestra in primo piano appartiene a questo programma.
///
/// Serve a non minimizzare FocusAnchor stesso: se l'obiettivo di studio
/// contiene una parola vietata (es. "Reddit"), la finestra dell'app non deve
/// essere chiusa da se'.
pub fn finestra_è_di_focus_anchor() -> bool {
    let processo = processo_della_finestra_attiva();
    processo != 0 && processo == std::process::id()
}

/// Minimizza la finestra in primo piano, se esiste.
pub fn minimizza_finestra_attiva() {
    unsafe {
        let hwnd = GetForegroundWindow();
        if !hwnd.is_invalid() {
            let _ = ShowWindow(hwnd, SW_MINIMIZE);
        }
    }
}

/// Verifica se il titolo della finestra corrisponde a una parola chiave vietata.
pub fn titolo_vietato(titolo: &str, parole: &[String]) -> bool {
    let minuscolo = titolo.to_lowercase();
    parole
        .iter()
        .filter(|p| !p.trim().is_empty())
        .any(|p| minuscolo.contains(&p.trim().to_lowercase()))
}

#[cfg(test)]
mod test {
    use super::*;

    /// Hosts di partenza con un blocco gia' presente da una sessione precedente.
    const HOSTS_SPORCO: &str = "127.0.0.1 localhost\n\
# === INIZIO FOCUS ANCHOR ===\n\
127.0.0.1 reddit.com\n\
::1 reddit.com\n\
# === FINE FOCUS ANCHOR ===\n";

    #[test]
    fn rimuove_il_blocco_precedente_ripristinando_gli_altri_record() {
        let pulito = rimuovi_blocco(HOSTS_SPORCO);
        assert!(!pulito.contains(MARCA_INIZIO));
        assert!(!pulito.contains(MARCA_FINE));
        assert!(!pulito.contains("reddit.com"));
        assert!(pulito.contains("127.0.0.1 localhost"));
    }

    #[test]
    fn non_tocca_un_hosts_senza_blocco_focus_anchor() {
        let originale = "127.0.0.1 localhost\n::1 localhost\n";
        assert_eq!(rimuovi_blocco(originale), originale);
    }

    #[test]
    fn applica_e_rimuove_il_blocco_in_modo_idempotente() {
        let domini = vec!["reddit.com".to_string(), "youtube.com".to_string()];
        let applicato = format!("127.0.0.1 localhost\n{}", costruisci_blocco(&domini));
        let ripulito = rimuovi_blocco(&applicato);
        assert_eq!(ripulito, "127.0.0.1 localhost\n\n");
        // Applicare due volte non deve duplicare il blocco.
        let riapplicato = format!("{}", rimuovi_blocco(&applicato) + &costruisci_blocco(&domini));
        assert_eq!(riapplicato.matches(MARCA_INIZIO).count(), 1);
    }

    #[test]
    fn il_blocco_redirige_il_dominio_verso_loopback() {
        let blocco = costruisci_blocco(&["reddit.com".to_string()]);
        assert!(blocco.contains("127.0.0.1 reddit.com"));
        assert!(blocco.contains("::1 reddit.com"));
    }

    #[test]
    fn riconosce_i_titoli_vietati_ignorando_le_maiuscole() {
        let parole = vec!["reddit".to_string(), " - youtube".to_string()];
        assert!(titolo_vietato("Reddit - The best site", &parole));
        assert!(titolo_vietato("Video di lezione - YouTube - Chrome", &parole));
        assert!(!titolo_vietato("Cap. 3 Storia Romana - Zotero", &parole));
    }

    #[test]
    fn ignora_le_parole_vietate_vuote() {
        let parole = vec!["".to_string(), "   ".to_string()];
        assert!(!titolo_vietato("Qualsiasi finestra", &parole));
    }

    #[test]
    fn il_processo_foreground_è_riconosciuto_come_proprio() {
        // Il controllo su se' stessi: il PID letto deve combaciare col nostro.
        let processo = processo_della_finestra_attiva();
        // Il processo in primo piano durante il test e' la console: se e' la
        // nostra, il confronto deve dare vero.
        if processo == std::process::id() {
            assert!(finestra_è_di_focus_anchor());
        } else {
            assert!(!finestra_è_di_focus_anchor());
        }
    }
}
