//! Browser incorporato per NotebookLM.
//!
//! NotebookLM e' una pagina web: per mostrarla dentro la finestra serve un
//! motore di rendering. Si usa WebView2 (Edge), che su Windows 11 e' gia'
//! installato, attraverso `wry`.
//!
//! Il profilo del browser sta in una cartella dell'app: il login a Google si
//! fa una volta dentro la finestra e resta.
//!
//! Limite noto: Google puo' rifiutare l'accesso nei browser incorporati. Se
//! succede, resta il pannello con la CLI (domande, fonti, flashcard) e il
//! pulsante per aprire la pagina nel browser normale.

use std::num::NonZeroIsize;

use wry::dpi::{PhysicalPosition, PhysicalSize, Position, Size};
use wry::raw_window_handle as rwh;
use wry::{Rect, WebContext, WebView, WebViewBuilder};

/// Un handle di finestra visto da wry.
///
/// Serve un tipo locale per implementare `HasWindowHandle`: il trait e'
/// esterno, quindi non si puo' implementare su `isize`.
struct Ospite(isize);

impl rwh::HasWindowHandle for Ospite {
    fn window_handle(&self) -> Result<rwh::WindowHandle<'_>, rwh::HandleError> {
        let hwnd = NonZeroIsize::new(self.0).ok_or(rwh::HandleError::Unavailable)?;
        let raw = rwh::RawWindowHandle::Win32(rwh::Win32WindowHandle::new(hwnd));
        // La struttura e' valida per tutta la durata della creazione.
        Ok(unsafe { rwh::WindowHandle::borrow_raw(raw) })
    }
}

/// La pagina web incorporata.
pub struct Pagina {
    webview: WebView,
}

impl Pagina {
    /// Crea il browser dentro la finestra e apre `url`.
    pub fn crea(
        finestra: isize,
        url: &str,
        cartella_profilo: &std::path::Path,
    ) -> Result<Self, String> {
        let mut contesto = WebContext::new(Some(cartella_profilo.to_path_buf()));
        let ospite = Ospite(finestra);
        let webview = WebViewBuilder::new_with_web_context(&mut contesto)
            .with_url(url)
            .build_as_child(&ospite)
            .map_err(|e| format!("Non riesco ad aprire NotebookLM nella finestra: {e}"))?;
        Ok(Self { webview })
    }

    /// Sposta e ridimensiona la pagina.
    pub fn posiziona(&self, x: i32, y: i32, larghezza: i32, altezza: i32) {
        let _ = self.webview.set_bounds(Rect {
            position: Position::Physical(PhysicalPosition::new(x, y)),
            size: Size::Physical(PhysicalSize::new(
                larghezza.max(1) as u32,
                altezza.max(1) as u32,
            )),
        });
    }

    /// Mostra o nasconde la pagina senza distruggerla.
    pub fn mostra(&self, visibile: bool) {
        let _ = self.webview.set_visible(visibile);
    }

    /// Apre un altro indirizzo nella stessa pagina.
    pub fn naviga(&self, url: &str) {
        let _ = self.webview.load_url(url);
    }

    /// Indirizzo attualmente aperto.
    pub fn indirizzo(&self) -> String {
        self.webview.url().unwrap_or_default()
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use wry::raw_window_handle::HasWindowHandle;

    /// Un handle nullo non deve produrre una finestra valida: wry lo
    /// rifiuterebbe in modo poco comprensibile.
    #[test]
    fn un_handle_nullo_non_e_una_finestra() {
        assert!(Ospite(0).window_handle().is_err());
    }
}
