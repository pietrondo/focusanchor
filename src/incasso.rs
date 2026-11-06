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

use windows::Win32::Foundation::{BOOL, CloseHandle, HWND, LPARAM, TRUE};
use windows::Win32::System::Diagnostics::ToolHelp::{
    CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W, TH32CS_SNAPPROCESS,
};
use windows::Win32::UI::WindowsAndMessaging::{
    EnumWindows, GetParent, GetWindowLongW, GetWindowThreadProcessId, IsWindowVisible, MoveWindow,
    SetParent, SetWindowLongW, ShowWindow, GWL_STYLE, SW_SHOW, SW_SHOWNORMAL, WS_CHILD, WS_POPUP,
};

thread_local! {
    /// Identificativo del processo cercato dall'enumerazione in corso.
    static PROCESSO: RefCell<u32> = const { RefCell::new(0) };
}

/// Cerca la finestra visibile di un processo.
fn cerca_per_processo(processo: u32) -> Option<HWND> {
    PROCESSO.with(|p| *p.borrow_mut() = processo);
    let mut trovata: Option<HWND> = None;
    unsafe {
        let _ = EnumWindows(
            Some(cerca_processo_callback),
            LPARAM(&mut trovata as *mut Option<HWND> as isize),
        );
    }
    trovata
}

/// Callback di enumerazione: si ferma alla prima finestra del processo.
unsafe extern "system" fn cerca_processo_callback(hwnd: HWND, lparam: LPARAM) -> BOOL {
    if !IsWindowVisible(hwnd).as_bool() {
        return TRUE;
    }
    let mut di: u32 = 0;
    let _ = GetWindowThreadProcessId(hwnd, Some(&mut di));
    if PROCESSO.with(|p| *p.borrow() == di) {
        let trovata = &mut *(lparam.0 as *mut Option<HWND>);
        *trovata = Some(hwnd);
        return BOOL(0); // ferma l'enumerazione
    }
    TRUE
}

/// Il processo di Zotero, se e' aperto.
///
/// Il nome dell'eseguibile e' l'unico modo sicuro: una finestra qualsiasi
/// che finisce con " - zotero" nel titolo non e' Zotero, e agganciarla
/// dentro la nostra finestra la farebbe sparire dallo schermo.
pub fn processo_zotero() -> Option<u32> {
    unsafe {
        let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0).ok()?;
        let _ = CloseHandle(snapshot);
        let mut voce = PROCESSENTRY32W {
            dwSize: std::mem::size_of::<PROCESSENTRY32W>() as u32,
            ..Default::default()
        };
        if Process32FirstW(snapshot, &mut voce).is_err() {
            return None;
        }
        loop {
            let nome = String::from_utf16_lossy(&voce.szExeFile);
            if nome.trim_end_matches('\0').eq_ignore_ascii_case("zotero.exe") {
                return Some(voce.th32ProcessID);
            }
            if Process32NextW(snapshot, &mut voce).is_err() {
                return None;
            }
        }
    }
}

/// La finestra principale di FocusAnchor.
///
/// Si cerca per processo, non per titolo: una finestra che ha "focusanchor"
/// nel titolo non deve diventare il padre di Zotero.
pub fn finestra_app() -> Option<HWND> {
    cerca_per_processo(std::process::id())
}

/// La finestra principale di Zotero.
pub fn finestra_zotero() -> Option<HWND> {
    cerca_per_processo(processo_zotero()?)
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
