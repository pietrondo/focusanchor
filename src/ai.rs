//! Assistente AI: spiega un passaggio usando OpenCode.
//!
//! Non chiama nessuna API direttamente: lancia `opencode run` con il modello
//! configurato, riusando l'autenticazione che l'utente ha gia'. Nessuna
//! chiave nel codice, nessuna dipendenza HTTP in piu'.
//!
//! La chiamata sta in un thread separato: puo' metterci dieci secondi e
//! l'interfaccia non deve congelarsi.

use std::os::windows::process::CommandExt;
use std::process::{Command, Stdio};
use std::sync::mpsc::{self, Receiver, Sender};

use crate::blocker::CREATE_NO_WINDOW;

/// Modello usato se la configurazione non ne indica uno.
pub const MODELLO_PREDEFINITO: &str = "opencode-go/deepseek-v4.1-flash";

/// Limite di caratteri del passaggio inviato.
pub const MASSIMO_CARATTERI: usize = 3000;

/// Esito di una richiesta all'assistente.
#[derive(Debug, Clone)]
pub enum Esito {
    /// Il modello ha risposto.
    Risposta(String),
    /// Qualcosa e' andato storto.
    Errore(String),
}

/// Stato dell'assistente AI.
pub struct Assistente {
    /// Canale su cui arriva la risposta dal thread.
    canale: Receiver<Esito>,
    /// Lato mittente, clonato nel thread.
    mittente: Sender<Esito>,
    /// Ultima risposta ricevuta.
    pub risposta: Option<String>,
    /// Vero mentre il modello sta pensando.
    pub in_corso: bool,
    /// Domanda a cui si riferisce la risposta.
    pub domanda: String,
}

impl Assistente {
    /// Crea l'assistente, senza nessuna richiesta in corso.
    pub fn nuovo() -> Self {
        let (mittente, canale) = mpsc::channel();
        Self {
            canale,
            mittente,
            risposta: None,
            in_corso: false,
            domanda: String::new(),
        }
    }

    /// Chiede al modello di spiegare un passaggio nel contesto di un esame.
    ///
    /// Se c'e' gia' una richiesta in corso non ne parte un'altra: due
    /// processi insieme rallentano tutto e consumano quota per niente.
    pub fn chiedi(&mut self, modello: &str, esame: &str, passaggio: &str) {
        if self.in_corso {
            return;
        }
        let passaggio = passaggio.trim();
        if passaggio.is_empty() {
            return;
        }
        let modello = if modello.trim().is_empty() {
            MODELLO_PREDEFINITO
        } else {
            modello.trim()
        };
        let prompt = componi_prompt(esame, passaggio);
        self.domanda = passaggio.chars().take(80).collect();
        self.risposta = None;
        self.in_corso = true;

        let mittente = self.mittente.clone();
        let modello = modello.to_string();
        std::thread::spawn(move || {
            let esito = esegui(&modello, &prompt);
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
        self.domanda.clear();
    }
}

/// Compone la richiesta inviata al modello.
pub fn componi_prompt(esame: &str, passaggio: &str) -> String {
    let esame = if esame.trim().is_empty() {
        "questo esame"
    } else {
        esame.trim()
    };
    let estratto = accorcia(passaggio);
    format!(
        "Sto preparando l'esame di {esame}. Spiegami in italiano, in modo chiaro e \
         concreto, questo passaggio dei miei testi. Massimo 120 parole, niente \
         premesse e niente ripetizioni.\n\n«{estratto}»"
    )
}

/// Taglia un testo troppo lungo a un confine di parola.
fn accorcia(testo: &str) -> String {
    if testo.chars().count() <= MASSIMO_CARATTERI {
        return testo.trim().to_string();
    }
    let tagliato: String = testo.chars().take(MASSIMO_CARATTERI).collect();
    match tagliato.rfind(' ') {
        Some(pos) => format!("{} […]", &tagliato[..pos]),
        None => format!("{tagliato} […]"),
    }
}

/// Esegue `opencode run` e restituisce la risposta.
///
/// Passa da `cmd /C` perche' OpenCode su Windows e' uno script `.cmd`: una
/// `Command::new("opencode")` diretta non lo troverebbe.
fn esegui(modello: &str, prompt: &str) -> Esito {
    let uscita = Command::new("cmd")
        .args([
            "/C", "opencode", "run", "-m", modello, "--auto", prompt,
        ])
        .creation_flags(CREATE_NO_WINDOW)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output();

    let uscita = match uscita {
        Ok(u) => u,
        Err(e) => {
            return Esito::Errore(format!(
                "Non riesco ad avviare OpenCode: {e}. Controlla che `opencode` sia installato."
            ))
        }
    };

    let testo = String::from_utf8_lossy(&uscita.stdout);
    let risposta = ripulisci(&testo);
    if risposta.is_empty() {
        let errore = String::from_utf8_lossy(&uscita.stderr);
        let errore = errore.trim();
        if errore.is_empty() {
            return Esito::Errore("Il modello non ha risposto.".into());
        }
        return Esito::Errore(format!("OpenCode: {}", errore.chars().take(400).collect::<String>()));
    }
    Esito::Risposta(risposta)
}

/// Toglie l'intestazione della sessione e i codici colore dal testo.
///
/// `opencode run` stampa una riga tipo `> build · modello` prima della
/// risposta: all'utente interessa solo il contenuto.
pub fn ripulisci(uscita: &str) -> String {
    uscita
        .lines()
        .filter(|riga| {
            let r = riga.trim();
            !(r.starts_with("> ") || r.is_empty() && false)
        })
        .map(|riga| rimuovi_ansi(riga))
        .collect::<Vec<_>>()
        .join("\n")
        .trim()
        .to_string()
}

/// Rimuove le sequenze di escape ANSI da una riga.
fn rimuovi_ansi(riga: &str) -> String {
    let mut risultato = String::with_capacity(riga.len());
    let mut caratteri = riga.chars().peekable();
    while let Some(c) = caratteri.next() {
        if c == '\u{1b}' {
            // Salta fino alla lettera finale della sequenza.
            for s in caratteri.by_ref() {
                if s.is_ascii_alphabetic() {
                    break;
                }
            }
            continue;
        }
        risultato.push(c);
    }
    risultato
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn il_prompt_contiene_esame_e_passaggio() {
        let p = componi_prompt("Storia romana", "La riforma di Tiberio Gracco");
        assert!(p.contains("Storia romana"));
        assert!(p.contains("La riforma di Tiberio Gracco"));
        assert!(p.contains("italiano"));
    }

    #[test]
    fn un_passaggio_lungo_viene_tagliato() {
        let lungo = "parola ".repeat(MASSIMO_CARATTERI);
        let p = componi_prompt("Storia", &lungo);
        assert!(p.contains("[…]"));
        assert!(p.chars().count() < lungo.chars().count());
    }

    #[test]
    fn l_intestazione_della_sessione_viene_tolta() {
        let uscita = "> build · deepseek-v4.1-flash\nLa risposta vera.\n";
        assert_eq!(ripulisci(uscita), "La risposta vera.");
    }

    #[test]
    fn i_codici_colore_vengono_tolti() {
        let uscita = "\u{1b}[31mrosso\u{1b}[0m normale";
        assert_eq!(ripulisci(uscita), "rosso normale");
    }

    /// Chiamata reale a OpenCode. Ignorata perche' richiede la CLI
    /// configurata e consuma quota: `cargo test ai -- --ignored`.
    #[test]
    #[ignore]
    fn opencode_risponde_davvero() {
        let esito = esegui(
            MODELLO_PREDEFINITO,
            "Rispondi in italiano con una sola frase: cosa e' la fotosintesi?",
        );
        match esito {
            Esito::Risposta(testo) => {
                println!("risposta: {testo}");
                assert!(!testo.trim().is_empty());
            }
            Esito::Errore(e) => panic!("errore: {e}"),
        }
    }
}
