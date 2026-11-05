//! Integrazione con NotebookLM: chiedere e ricevere la spiegazione.
//!
//! Il flusso reale di studio e': copio un pezzo del riassunto che sto
//! leggendo, lo incollo nella chat di NotebookLM dell'esame (dove ho gia'
//! caricato tutti i testi) e mi faccio spiegare meglio i passaggi ostici.
//!
//! Il punto di attrito e' compose ogni volta la domanda giusta e ricordare
//! per quale esame si sta chiedendo. Questo modulo prepara il messaggio
//! pronto: l'utente lo incolla e preme Invio.
//!
//! Il secondo attrito e' la risposta: NotebookLM la scrive nel browser e
//! l'app non la vede. `leggi_risposta` la recupera dalla chat aperta
//! dell'utente, cosi' la spiegazione torna dove si studia e diventa
//! materiale per il ripasso.

use std::os::windows::process::CommandExt;
use std::process::Command;

use crate::blocker::CREATE_NO_WINDOW;

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
