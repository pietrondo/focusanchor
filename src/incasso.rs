//! Agganciare la finestra di un altro programma dentro la nostra.
//!
//! Serve per Zotero: e' un'applicazione nativa, e Windows permette di
//! renderla figlia della nostra finestra con `SetParent`. Non e' un'API
//! pensata per questo (Microsoft non documenta l'aggancio fra processi
//! diversi come supportato), ma funziona, ed e' lo stesso trucco dei
//! launcher che incorporano altri programmi.
//!
//! Due regole non negoziabili:
//! - si deve poter staccare con un pulsante;
//! - all'uscita la finestra va ripristinata, altrimenti resta figlia di una
//!   finestra che non esiste piu' e sparisce dallo schermo.

use std::cell::RefCell;

use windows::Win32::Foundation::{BOOL, HWND, LPARAM, TRUE};
use windows::Win32::UI::WindowsAndMessaging::{
    EnumWindows, GetParent, GetWindowLongW, GetWindowTextW, IsWindowVisible, MoveWindow, SetParent,
    SetWindowLongW, ShowWindow, GWL_STYLE, SW_SHOW, SW_SHOWNORMAL, WS_CHILD, WS_POPUP,
};

/// Titolo massimo letto da una finestra.
const MAX_TITOLO: usize = 512;

thread_local! {
    /// Frammento di titolo cercato dall'enumerazione in corso.
    ///
    /// Il callback di `EnumWindows` non riceve argomenti liberi: il
    /// frammento passa da qui.
    static FRAMMENTO: RefCell<String> = const { RefCell::new(String::new()) };
}

/// Cerca una finestra visibile il cui titolo contiene il frammento.
pub fn cerca(frammento: &str) -> Option<HWND> {
    FRAMMENTO.with(|f| *f.borrow_mut() = frammento.to_lowercase());
    let mut trovata: Option<HWND> = None;
    unsafe {
        let _ = EnumWindows(
            Some(cerca_callback),
            LPARAM(&mut trovata as *mut Option<HWND> as isize),
        );
    }
    trovata
}

/// Callback di enumerazione: si ferma alla prima finestra che combacia.
unsafe extern "system" fn cerca_callback(hwnd: HWND, lparam: LPARAM) -> BOOL {
    if !IsWindowVisible(hwnd).as_bool() {
        return TRUE;
    }
    let mut buffer = [0u16; MAX_TITOLO];
    let lunghezza = GetWindowTextW(hwnd, &mut buffer);
    if lunghezza <= 0 {
        return TRUE;
    }
    let titolo = String::from_utf16_lossy(&buffer[..lunghezza as usize]).to_lowercase();
    let combacia = FRAMMENTO.with(|f| titolo.contains(&*f.borrow()));
    if combacia {
        let trovata = &mut *(lparam.0 as *mut Option<HWND>);
        *trovata = Some(hwnd);
        return BOOL(0); // ferma l'enumerazione
    }
    TRUE
}

/// La finestra principale di FocusAnchor.
pub fn finestra_app() -> Option<HWND> {
    cerca("focusanchor")
}

/// La finestra principale di Zotero.
pub fn finestra_zotero() -> Option<HWND> {
    cerca(" - zotero")
}

/// Aggancia una finestra dentro la nostra, alla posizione indicata.
///
/// Restituisce `false` se Windows rifiuta: in quel caso la finestra resta
/// com'era e non c'e' niente da ripristinare.
pub fn aggancia(figlio: HWND, genitore: HWND, x: i32, y: i32, larghezza: i32, altezza: i32) -> bool {
    unsafe {
        let stile = GetWindowLongW(figlio, GWL_STYLE);
        // WS_POPUP e WS_CHILD sono mutuamente esclusivi.
        let nuovo = (stile & !(WS_POPUP.0 as i32)) | WS_CHILD.0 as i32;
        let _ = SetWindowLongW(figlio, GWL_STYLE, nuovo);
        if SetParent(figlio, genitore).is_err() {
            return false;
        }
        let _ = MoveWindow(figlio, x, y, larghezza, altezza, true);
        let _ = ShowWindow(figlio, SW_SHOW);
    }
    true
}

/// Riporta la finestra nella posizione indicata del riquadro.
pub fn posiziona(figlio: HWND, x: i32, y: i32, larghezza: i32, altezza: i32) {
    unsafe {
        let _ = MoveWindow(figlio, x, y, larghezza, altezza, true);
    }
}

/// Stacca la finestra: torna indipendente e visibile.
pub fn stacca(figlio: HWND) {
    unsafe {
        let _ = SetParent(figlio, None);
        let stile = GetWindowLongW(figlio, GWL_STYLE);
        let nuovo = (stile & !(WS_CHILD.0 as i32)) | WS_POPUP.0 as i32;
        let _ = SetWindowLongW(figlio, GWL_STYLE, nuovo);
        let _ = ShowWindow(figlio, SW_SHOWNORMAL);
    }
}

/// Vero se la finestra risulta agganciata a un'altra.
pub fn e_agganciata(figlio: HWND) -> bool {
    unsafe { GetParent(figlio).map(|p| !p.is_invalid()).unwrap_or(false) }
}

/// Avvia Zotero, se non e' gia' aperto.
pub fn avvia_zotero() -> Result<(), String> {
    use std::os::windows::process::CommandExt;
    const PERCORSI: [&str; 2] = [
        r"C:\Program Files\Zotero\zotero.exe",
        r"C:\Program Files (x86)\Zotero\zotero.exe",
    ];
    for percorso in PERCORSI {
        if std::path::Path::new(percorso).exists() {
            std::process::Command::new(percorso)
                .creation_flags(crate::blocker::CREATE_NO_WINDOW)
                .spawn()
                .map_err(|e| format!("Non riesco ad avviare Zotero: {e}"))?;
            return Ok(());
        }
    }
    Err("Non trovo Zotero in C:\\Program Files. Aprilo a mano e riprova.".into())
}
