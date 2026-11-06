//! Assistente AI: spiega un passaggio usando OpenCode.
//!
//! Non chiama nessuna API direttamente: lancia `opencode run` con il modello
//! configurato, riusando l'autenticazione che l'utente ha gia'. Nessuna
//! chiave nel codice, nessuna dipendenza HTTP in piu'.
//!
//! La chiamata sta in un thread separato: puo' metterci dieci secondi e
//! l'interfaccia non deve congelarsi.

use std::io::Write;
use std::os::windows::process::CommandExt;
use std::process::{Command, Stdio};
use std::sync::mpsc::{self, Receiver, Sender};

use crate::blocker::CREATE_NO_WINDOW;

/// Modello usato se la configurazione non ne indica uno.
pub const MODELLO_PREDEFINITO: &str = "opencode-go/deepseek-v4.1-flash";

/// Limite di caratteri del passaggio inviato.
pub const MASSIMO_CARATTERI: usize = 3000;

/// Quante sottolineature dell'esame si passano come contesto.
///
/// Poche e scelte: il modello deve capire come il corso tratta l'argomento,
/// non ricevere un libro intero.
pub const MASSIMO_CONTESTO: usize = 3;

/// Lunghezza massima di una singola sottolineatura nel contesto.
const CARATTERI_CONTESTO: usize = 220;

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
    /// `contesto` sono le sottolineature dell'esame lette da Zotero: danno al
    /// modello il modo in cui il corso tratta l'argomento, che e' la
    /// differenza fra una spiegazione generica e una utile.
    ///
    /// Se c'e' gia' una richiesta in corso non ne parte un'altra: due
    /// processi insieme rallentano tutto e consumano quota per niente.
    pub fn chiedi(&mut self, modello: &str, esame: &str, passaggio: &str, contesto: &[String]) {
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
        let prompt = componi_prompt(esame, passaggio, contesto);
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
pub fn componi_prompt(esame: &str, passaggio: &str, contesto: &[String]) -> String {
    let esame = if esame.trim().is_empty() {
        "questo esame"
    } else {
        esame.trim()
    };
    let estratto = accorcia(passaggio);
    let mut prompt = format!(
        "Sto preparando l'esame di {esame}. Spiegami in italiano, in modo chiaro e \
         concreto, questo passaggio dei miei testi. Massimo 120 parole, niente \
         premesse e niente ripetizioni.\n\n«{estratto}»"
    );

    // Le sottolineature dicono come il corso tratta l'argomento: senza,
    // la spiegazione e' corretta ma scollegata dal programma.
    let righe: Vec<String> = contesto
        .iter()
        .map(|t| t.trim())
        .filter(|t| !t.is_empty())
        .take(MASSIMO_CONTESTO)
        .map(|t| format!("- {}", taglia(t, CARATTERI_CONTESTO)))
        .collect();
    if !righe.is_empty() {
        prompt.push_str(
            "\n\nNel corso ho sottolineato questo, tienine conto:\n",
        );
        prompt.push_str(&righe.join("\n"));
    }
    prompt
}

/// Taglia un testo a un confine di parola, senza segnale di taglio.
fn taglia(testo: &str, massimo: usize) -> String {
    if testo.chars().count() <= massimo {
        return testo.to_string();
    }
    let tagliato: String = testo.chars().take(massimo).collect();
    match tagliato.rfind(' ') {
        Some(pos) => tagliato[..pos].to_string(),
        None => tagliato,
    }
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

/// Vero se il nome del modello e' solo il formato `provider/modello`.
///
/// Il nome va sulla riga di comando di `cmd`: se contenesse un `&` o una
/// virgoletta, `cmd` lo prenderebbe per un altro comando.
fn modello_sicuro(modello: &str) -> bool {
    !modello.is_empty()
        && modello
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '/' | '-' | '_' | '.' | '#'))
}

/// Esegue `opencode run` e restituisce la risposta.
///
/// Passa da `cmd /C` perche' OpenCode su Windows e' uno script `.cmd`: una
/// `Command::new("opencode")` diretta non lo troverebbe.
///
/// Il prompt va da stdin e non sulla riga di comando: `cmd` riaparse la
/// riga e un `&` o una virgoletta nel testo copiato da un PDF diventerebbero
/// un comando.
fn esegui(modello: &str, prompt: &str) -> Esito {
    if !modello_sicuro(modello) {
        return Esito::Errore(format!("Nome del modello non valido: {modello}"));
    }
    let mut figlio = match Command::new("cmd")
        .args(["/C", "opencode", "run", "-m", modello, "--auto"])
        .creation_flags(CREATE_NO_WINDOW)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
    {
        Ok(f) => f,
        Err(e) => {
            return Esito::Errore(format!(
                "Non riesco ad avviare OpenCode: {e}. Controlla che `opencode` sia installato."
            ))
        }
    };
    if let Some(mut stdin) = figlio.stdin.take() {
        let _ = stdin.write_all(prompt.as_bytes());
    }

    let uscita = match figlio.wait_with_output() {
        Ok(u) => u,
        Err(e) => return Esito::Errore(format!("OpenCode non ha risposto: {e}")),
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
        .enumerate()
        .filter(|(indice, riga)| !(*indice == 0 && riga.trim().starts_with("> ")))
        .map(|(_, riga)| rimuovi_ansi(riga))
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
        let p = componi_prompt("Storia romana", "La riforma di Tiberio Gracco", &[]);
        assert!(p.contains("Storia romana"));
        assert!(p.contains("La riforma di Tiberio Gracco"));
        assert!(p.contains("italiano"));
    }

    #[test]
    fn le_sottolineature_entrano_nel_prompt() {
        let contesto = vec![
            "La riforma agraria del 133 a.C.".to_string(),
            "   ".to_string(),
            "Il conflitto con il senato".to_string(),
        ];
        let p = componi_prompt("Storia romana", "Tiberio Gracco", &contesto);
        assert!(p.contains("La riforma agraria del 133 a.C."));
        assert!(p.contains("Il conflitto con il senato"));
        // Il contesto e' limitato: non deve diventare un libro.
        let molte: Vec<String> = (0..10).map(|i| format!("sottolineatura {i}")).collect();
        let p = componi_prompt("Storia", "passaggio", &molte);
        assert!(p.contains("sottolineatura 2"));
        assert!(!p.contains("sottolineatura 3"));
    }

    #[test]
    fn un_passaggio_lungo_viene_tagliato() {
        let lungo = "parola ".repeat(MASSIMO_CARATTERI);
        let p = componi_prompt("Storia", &lungo, &[]);
        assert!(p.contains("[…]"));
        assert!(p.chars().count() < lungo.chars().count());
    }

    #[test]
    fn l_intestazione_della_sessione_viene_tolta() {
        let uscita = "> build · deepseek-v4.1-flash\nLa risposta vera.\n";
        assert_eq!(ripulisci(uscita), "La risposta vera.");
    }

    #[test]
    fn ripulisci_non_elimina_blockquote_della_risposta() {
        let uscita = "> build · deepseek-v4.1-flash\n> Citazione rilevante.\nTesto successivo.";
        assert_eq!(
            ripulisci(uscita),
            "> Citazione rilevante.\nTesto successivo."
        );
    }

    #[test]
    fn i_codici_colore_vengono_tolti() {
        let uscita = "\u{1b}[31mrosso\u{1b}[0m normale";
        assert_eq!(ripulisci(uscita), "rosso normale");
    }

    /// Il nome del modello va sulla riga di comando di `cmd`: i caratteri
    /// che `cmd` prende per un separatore non possono passare.
    #[test]
    fn il_nome_del_modello_deve_essere_sicuro() {
        assert!(modello_sicuro("opencode-go/deepseek-v4.1-flash"));
        assert!(modello_sicuro("anthropic/claude-sonnet-4#thinking"));
        assert!(!modello_sicuro(""));
        assert!(!modello_sicuro("modello & calc"));
        assert!(!modello_sicuro("modello|calc"));
        assert!(!modello_sicuro("modello\""));
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
