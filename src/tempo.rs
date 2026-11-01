//! Tempo passato su ogni documento, letto dalla finestra in primo piano.
//!
//! I minuti di sessione sanno dire quanto hai studiato, non *dove*: tre ore
//! sul manuale e tre ore sul riassunto sono lo stesso numero e due lavori
//! diversi. Qui si guarda il titolo della finestra in primo piano e si
//! attribuisce il tempo al documento che quel titolo contiene.
//!
//! Nessun aggancio oltre a quello che serve: niente hook, niente servizi.
//! Solo `GetForegroundWindow` una volta al secondo.

use std::time::{Duration, Instant};

use crate::state::StatoApp;
use crate::zotero::Documento;

/// Ogni quanto si guarda la finestra in primo piano.
pub const INTERVALLO: Duration = Duration::from_secs(1);

/// Chiave del tempo passato su riassunti che non sono documenti di Zotero.
///
/// Un riassunto aperto nel browser o in Word non ha un titolo di Zotero a cui
/// agganciarsi: si riconosce dal nome della finestra e finisce qui.
pub const CHIAVE_RIASSUNTI: &str = "Riassunti e appunti";

/// Parole che fanno riconoscere un riassunto dal titolo della finestra.
///
/// Sono quelle che compaiono davvero nei nomi dei file di studio: "docsity
/// riassunto di…", "appunti moduli A e B", "dispensa di…".
const PAROLE_RIASSUNTO: &[&str] = &[
    "riassunt",
    "appunt",
    "dispens",
    "sintes",
    "schema",
    "mappa concettuale",
];

/// Titolo del documento che quella finestra sta mostrando, se si capisce.
///
/// Vince il titolo piu' lungo che combacia: "Storia romana vol. 2" non deve
/// sparire dentro "Storia romana".
pub fn classifica(finestra: &str, documenti: &[Documento]) -> Option<String> {
    let attiva = crate::zotero::normalizza(finestra);
    if attiva.len() < 8 {
        return None;
    }
    let migliore = documenti
        .iter()
        .filter(|documento| {
            let titolo = crate::zotero::normalizza(&documento.titolo);
            titolo.len() >= 8 && (attiva.contains(&titolo) || titolo.contains(&attiva))
        })
        .max_by_key(|documento| crate::zotero::normalizza(&documento.titolo).len());

    if let Some(documento) = migliore {
        return Some(documento.titolo.clone());
    }
    // Fuori da Zotero il titolo resta l'unica traccia: se parla di riassunti
    // si conta li', altrimenti quel tempo non e' attribuibile a niente.
    if e_riassunto(&attiva) {
        return Some(CHIAVE_RIASSUNTI.to_string());
    }
    None
}

/// Vero se quel titolo e' un riassunto e non un testo d'esame.
pub fn e_riassunto(titolo: &str) -> bool {
    if titolo == CHIAVE_RIASSUNTI {
        return true;
    }
    let normale = crate::zotero::normalizza(titolo);
    PAROLE_RIASSUNTO
        .iter()
        .any(|parola| normale.contains(parola))
}

impl StatoApp {
    /// Attribuisce al documento in primo piano il tempo dall'ultimo campione.
    ///
    /// Si misura il tempo reale e non il numero di campioni: l'interfaccia
    /// ridisegna a frequenza variabile, e contare i giri sottostimerebbe.
    /// L'orologio si riazzera a ogni chiamata, altrimenti una pausa di cinque
    /// minuti verrebbe accreditata in blocco al primo campione dopo la pausa.
    pub fn campiona_tempo(&mut self) {
        let adesso = Instant::now();
        let passato = adesso.saturating_duration_since(self.ultimo_campione);
        self.ultimo_campione = adesso;

        if self.sessione.is_none() || self.pausa_residua().is_some() || passato < INTERVALLO {
            return;
        }
        let finestra = crate::finestre::titolo_in_primo_piano();
        // Prima l'estensione, che sa la chiave esatta del documento; il titolo
        // della finestra resta il ripiego per chi non la installa.
        let da_estensione = crate::zotero_attivo::leggi_attivo(&self.config.cartella_zotero).and_then(|attivo| {
            crate::zotero_attivo::classifica_attivo(&attivo, &self.documenti_esame)
                // Se la chiave non e' fra i documenti dell'esame, il titolo che
                // l'estensione riporta e' comunque piu' affidabile della finestra.
                .or_else(|| classifica(&attivo.titolo, &self.documenti_esame))
        });
        let chiave = da_estensione.or_else(|| classifica(&finestra, &self.documenti_esame));
        if let Some(chiave) = chiave {
            *self.tempo_documenti.entry(chiave).or_insert(0) += passato.as_secs() as u32;
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    fn documento(titolo: &str) -> Documento {
        Documento {
            chiave: "ABCD1234".to_string(),
            chiave_pdf: Some("PDFKEY12".to_string()),
            titolo: titolo.to_string(),
            tipo: "book".to_string(),
            data_aggiunta: "2026-10-01 10:00:00".to_string(),
        }
    }

    #[test]
    fn riconosce_il_documento_dal_titolo_della_finestra() {
        let documenti = vec![documento("Storia romana"), documento("Letteratura italiana")];
        let esito = classifica("Storia romana - Zotero", &documenti);
        assert_eq!(esito.as_deref(), Some("Storia romana"));
    }

    #[test]
    fn il_titolo_piu_specifico_vince() {
        let documenti = vec![documento("Storia romana"), documento("Storia romana vol. 2")];
        let esito = classifica("Storia romana vol. 2 - Zotero", &documenti);
        assert_eq!(esito.as_deref(), Some("Storia romana vol. 2"));
    }

    #[test]
    fn un_riassunto_fuori_da_zotero_finisce_nei_riassunti() {
        let documenti = vec![documento("Manuale di storia romana")];
        let esito = classifica("riassunto_storia_romana.pdf - Adobe Acrobat", &documenti);
        assert_eq!(esito.as_deref(), Some(CHIAVE_RIASSUNTI));
    }

    #[test]
    fn una_finestra_qualsiasi_non_ruba_tempo() {
        let documenti = vec![documento("Manuale di storia romana")];
        assert!(classifica("Nuova scheda - Google Chrome", &documenti).is_none());
        // Titoli troppo corti non devono combaciare con niente.
        assert!(classifica("Appunti", &documenti).is_none());
    }

    #[test]
    fn l_estensione_riconosce_il_documento_dalla_chiave() {
        let documenti = vec![documento("Manuale di storia romana")];
        let attivo = crate::zotero_attivo::Attivo {
            chiave: "PDFKEY12".to_string(),
            chiave_padre: "ABCD1234".to_string(),
            titolo: "Manuale di storia romana".to_string(),
        };
        assert_eq!(
            crate::zotero_attivo::classifica_attivo(&attivo, &documenti).as_deref(),
            Some("Manuale di storia romana")
        );
        // Una chiave di un altro esame non attribuisce niente.
        let estraneo = crate::zotero_attivo::Attivo {
            chiave: "ZZZZ9999".to_string(),
            chiave_padre: "YYYY8888".to_string(),
            titolo: "Altro".to_string(),
        };
        assert!(crate::zotero_attivo::classifica_attivo(&estraneo, &documenti).is_none());
    }

    #[test]
    fn i_riassunti_si_riconoscono_dal_nome() {
        assert!(e_riassunto("docsity-riassunto-del-manuale-di-storia-romana"));
        assert!(e_riassunto("Appunti moduli A e B"));
        assert!(e_riassunto(CHIAVE_RIASSUNTI));
        assert!(!e_riassunto("Manuale di storia romana"));
    }
}
