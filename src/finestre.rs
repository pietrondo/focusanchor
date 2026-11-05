//! Rilevamento delle finestre Windows aperte.
//!
//! Serve a due scopi: ignorare la finestra di FocusAnchor stessa nella
//! guardia anti-distrazione, e trovare il PDF che l'utente sta leggendo
//! su Zotero.

use windows::Win32::Foundation::{BOOL, HWND, LPARAM, TRUE};
use windows::Win32::UI::WindowsAndMessaging::{
    EnumWindows, GetForegroundWindow, GetWindowTextW, IsWindowVisible,
};

/// Suffisso del titolo della finestra principale di Zotero.
pub const SUFFISSO_ZOTERO: &str = " - Zotero";

/// Titolo della finestra in primo piano, se esiste.
pub fn titolo_in_primo_piano() -> String {
    titolo_di(unsafe { GetForegroundWindow() })
}

/// Legge il titolo della finestra indicata.
fn titolo_di(hwnd: HWND) -> String {
    unsafe {
        let mut buffer = [0u16; 512];
        let lunghezza = GetWindowTextW(hwnd, &mut buffer);
        if lunghezza <= 0 {
            return String::new();
        }
        String::from_utf16_lossy(&buffer[..lunghezza as usize])
    }
}

/// Cerca il titolo del PDF aperto su Zotero.
///
/// Zotero aggiunge il suffisso " - Zotero" al titolo della finestra: il testo
/// che precede e' il documento che l'utente sta leggendo in quel momento.
pub fn pdf_aperto_su_zotero() -> Option<String> {
    finestre().into_iter().find_map(|titolo| {
        let t = titolo.trim();
        t.strip_suffix(SUFFISSO_ZOTERO)
            .map(|nome| nome.trim().to_string())
            .filter(|nome| !nome.is_empty())
    })
}

/// Elenco dei titoli di tutte le finestre visibili con un testo.
pub fn finestre() -> Vec<String> {
    let mut raccolta: Vec<String> = Vec::new();
    unsafe {
        let _ = EnumWindows(
            Some(callback),
            LPARAM(&mut raccolta as *mut Vec<String> as isize),
        );
    }
    raccolta.retain(|t| !t.trim().is_empty());
    raccolta
}

/// Callback di enumerazione: raccoglie i titoli delle finestre visibili.
unsafe extern "system" fn callback(hwnd: HWND, lparam: LPARAM) -> BOOL {
    if !IsWindowVisible(hwnd).as_bool() {
        return TRUE;
    }
    let titolo = titolo_di(hwnd);
    if titolo.trim().is_empty() {
        return TRUE;
    }
    // Il parametro e' il puntatore al vettore di raccolta.
    let raccolta = &mut *(lparam.0 as *mut Vec<String>);
    raccolta.push(titolo);
    TRUE
}

/// Dimensione dello schermo principale, in pixel fisici.
pub fn schermo() -> (f32, f32) {
    use windows::Win32::UI::WindowsAndMessaging::{
        GetSystemMetrics, SM_CXSCREEN, SM_CYSCREEN,
    };
    unsafe {
        let larghezza = GetSystemMetrics(SM_CXSCREEN) as f32;
        let altezza = GetSystemMetrics(SM_CYSCREEN) as f32;
        (larghezza.max(640.0), altezza.max(480.0))
    }
}

/// Area utile dello schermo principale, quella senza la barra delle
/// applicazioni.
///
/// Restituisce `(larghezza, altezza)` in pixel fisici. Se Windows non risponde
/// si ripiega sull'intero schermo.
pub fn area_utile() -> (f32, f32) {
    use windows::Win32::Foundation::RECT;
    use windows::Win32::UI::WindowsAndMessaging::{
        SystemParametersInfoW, SPI_GETWORKAREA,
    };
    let mut rettangolo = RECT::default();
    let esito = unsafe {
        SystemParametersInfoW(
            SPI_GETWORKAREA,
            0,
            Some(&mut rettangolo as *mut _ as *mut core::ffi::c_void),
            Default::default(),
        )
    };
    if esito.is_err() {
        return schermo();
    }
    let larghezza = (rettangolo.right - rettangolo.left) as f32;
    let altezza = (rettangolo.bottom - rettangolo.top) as f32;
    if larghezza < 320.0 || altezza < 240.0 {
        return schermo();
    }
    (larghezza, altezza)
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn l_enumerazione_ritorna_titoli_non_vuoti() {
        for titolo in finestre() {
            assert!(!titolo.trim().is_empty());
        }
    }
}
