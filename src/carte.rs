//! Carte di ripasso di un esame, con ripetizione dilazionata.
//!
//! Prima le domande a tradimento finivano in due posti: un file Markdown per
//! leggerle e un file tabulato per Anki, che e' un altro programma. Cosi' il
//! ripasso viveva fuori dall'app e ogni volta il ciclo si spezzava.
//!
//! Qui la carta e' l'unita' di memoria: domanda, risposta e quando torna. La
//! memoria e' un file per esame (`esame_<nome>.json`), cosi' un corso non
//! mescola le sue carte con quelle degli altri.
//!
//! La programmazione e' un SM-2 semplificato: non serve l'algoritmo completo,
//! serve che una carta sbagliata torni subito e una saputa torni sempre piu'
//! tardi.

use std::fs;
use std::path::PathBuf;

use chrono::{Local, NaiveDate};
use serde::{Deserialize, Serialize};

/// Quanto hai saputo rispondere.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Giudizio {
    /// Non lo sapevo: torna subito.
    Male,
    /// Piu' o meno: torna fra qualche giorno.
    Medio,
    /// Lo sapevo: intervallo piu' lungo.
    Bene,
}

/// Una carta: la domanda che ti e' stata fatta e la risposta che hai scritto.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Carta {
    /// Il passaggio da spiegare.
    pub domanda: String,
    /// La tua spiegazione.
    pub risposta: String,
    /// Giorno in cui la carta torna, formato `AAAA-MM-GG`.
    ///
    /// Il formato e' a larghezza fissa, quindi si confronta come stringa.
    pub prossima: String,
    /// Giorni fra un ripasso e l'altro.
    pub intervallo: u32,
    /// Quanto la ricordi: 1.3 (mai) a 2.8 (sempre).
    pub facilita: f64,
    /// Ripassi andati bene di fila.
    pub serie: u32,
    /// Volte in cui non la sapevi.
    pub errori: u32,
}

/// Tutto quello che l'app sa di un esame, oltre ai tempi.
#[derive(Serialize, Deserialize, Default, Clone, Debug)]
pub struct MemoriaEsame {
    /// Le carte, nell'ordine in cui sono nate.
    #[serde(default)]
    pub carte: Vec<Carta>,
}

/// Percorso della memoria di un esame.
pub fn percorso(esame: &str) -> PathBuf {
    crate::config::cartella_dati().join(format!(
        "esame_{}.json",
        crate::archivi::nome_file_sicuro(esame)
    ))
}

/// Carica la memoria di un esame; se manca o e' rotta, riparte da vuota.
pub fn carica(esame: &str) -> MemoriaEsame {
    let Ok(testo) = fs::read_to_string(percorso(esame)) else {
        return MemoriaEsame::default();
    };
    serde_json::from_str(&testo).unwrap_or_default()
}

/// Scrive la memoria di un esame su disco.
pub fn salva(esame: &str, memoria: &MemoriaEsame) -> Result<(), String> {
    let json = serde_json::to_string_pretty(memoria)
        .map_err(|e| format!("Memoria non serializzabile: {e}"))?;
    fs::write(percorso(esame), json)
        .map_err(|e| format!("Non riesco a salvare le carte: {e}"))
}

impl MemoriaEsame {
    /// Indici delle carte da ripassare oggi.
    pub fn scadute(&self, oggi: &str) -> Vec<usize> {
        self.carte
            .iter()
            .enumerate()
            .filter(|(_, carta)| carta.prossima.as_str() <= oggi && !carta.risposta.trim().is_empty())
            .map(|(indice, _)| indice)
            .collect()
    }

    /// Vero se una carta uguale esiste gia'.
    pub fn contiene(&self, domanda: &str) -> bool {
        self.carte
            .iter()
            .any(|carta| carta.domanda.trim() == domanda.trim())
    }

    /// Prossima carta da ripassare con indice successivo a `dopo`.
    ///
    /// Serve al pannello per avanzare senza ripescare la carta appena
    /// giudicata: se l'hai sbagliata resta scaduta oggi, ma non deve
    /// riapparire subito nella stessa passata.
    pub fn prossima_scaduta(&self, oggi: &str, dopo: Option<usize>) -> Option<usize> {
        let inizio = dopo.map(|indice| indice + 1).unwrap_or(0);
        self.scadute(oggi).into_iter().find(|indice| *indice >= inizio)
    }
}

impl crate::state::StatoApp {
    /// Allinea la memoria delle carte all'esame scelto.
    ///
    /// Si legge da disco solo quando l'esame cambia: farlo a ogni frame
    /// significherebbe una lettura di file sessanta volte al secondo.
    pub fn sincronizza_memoria(&mut self) {
        let esame = self.nome_esame();
        if esame != self.memoria_esame {
            self.memoria = carica(&esame);
            self.memoria_esame = esame;
            self.ripasso_indice = None;
            self.ripasso_risposta_visibile = false;
        }
    }
}

/// Aggiunge una carta presa da una risposta alle domande a tradimento.
///
/// Se la stessa domanda c'e' gia' non si duplica: la risposta vecchia resta,
/// altrimenti un passaggio studiato due volte tornerebbe due volte.
pub fn aggiungi(esame: &str, domanda: &str, risposta: &str) -> Result<(), String> {
    let domanda = domanda.trim();
    let risposta = risposta.trim();
    if domanda.is_empty() || risposta.is_empty() {
        return Ok(());
    }
    let mut memoria = carica(esame);
    if memoria.contiene(domanda) {
        return Ok(());
    }
    memoria.carte.push(Carta {
        domanda: domanda.to_string(),
        risposta: risposta.to_string(),
        // Una carta nuova si ripassa subito: e' appena nata da un dubbio.
        prossima: oggi(),
        intervallo: 0,
        facilita: 2.5,
        serie: 0,
        errori: 0,
    });
    salva(esame, &memoria)
}

/// Aggiorna la carta secondo il giudizio dato.
pub fn ripassa(carta: &mut Carta, giudizio: Giudizio, giorno: NaiveDate) {
    match giudizio {
        Giudizio::Male => {
            carta.serie = 0;
            carta.errori += 1;
            carta.intervallo = 0;
            carta.facilita = (carta.facilita - 0.2).max(1.3);
        }
        Giudizio::Medio => {
            carta.intervallo = ((carta.intervallo as f64 * 1.2).round() as u32).max(1);
            carta.facilita = (carta.facilita - 0.05).max(1.3);
        }
        Giudizio::Bene => {
            carta.intervallo = match carta.serie {
                0 => 1,
                1 => 3,
                _ => ((carta.intervallo as f64 * carta.facilita).round() as u32).max(1),
            };
            carta.serie += 1;
            carta.facilita = (carta.facilita + 0.1).min(2.8);
        }
    }
    carta.prossima = (giorno + chrono::Duration::days(carta.intervallo as i64))
        .format("%Y-%m-%d")
        .to_string();
}

/// Data di oggi in formato `AAAA-MM-GG`.
pub fn oggi() -> String {
    Local::now().format("%Y-%m-%d").to_string()
}

#[cfg(test)]
mod test {
    use super::*;

    fn giorno(testo: &str) -> NaiveDate {
        NaiveDate::parse_from_str(testo, "%Y-%m-%d").expect("data")
    }

    fn carta_nuova() -> Carta {
        Carta {
            domanda: "La riforma di Tiberio Gracco".to_string(),
            risposta: "Ridistribuzione dell'agro pubblico".to_string(),
            prossima: "2026-11-01".to_string(),
            intervallo: 0,
            facilita: 2.5,
            serie: 0,
            errori: 0,
        }
    }

    #[test]
    fn una_carta_saputa_torna_sempre_piu_tardi() {
        let mut carta = carta_nuova();
        let oggi = giorno("2026-11-01");

        ripassa(&mut carta, Giudizio::Bene, oggi);
        assert_eq!(carta.intervallo, 1);
        assert_eq!(carta.prossima, "2026-11-02");

        ripassa(&mut carta, Giudizio::Bene, giorno("2026-11-02"));
        assert_eq!(carta.intervallo, 3);
        assert_eq!(carta.prossima, "2026-11-05");

        // Dalla terza volta in poi cresce moltiplicando.
        ripassa(&mut carta, Giudizio::Bene, giorno("2026-11-05"));
        assert!(carta.intervallo > 3);
        assert_eq!(carta.serie, 3);
    }

    #[test]
    fn una_carta_sbagliata_torna_subito() {
        let mut carta = carta_nuova();
        carta.serie = 4;
        carta.intervallo = 20;
        let oggi = giorno("2026-11-01");

        ripassa(&mut carta, Giudizio::Male, oggi);
        assert_eq!(carta.intervallo, 0);
        assert_eq!(carta.prossima, "2026-11-01");
        assert_eq!(carta.serie, 0);
        assert_eq!(carta.errori, 1);
        // La facilita' scende ma non sotto il minimo.
        assert!(carta.facilita < 2.5 && carta.facilita >= 1.3);
    }

    #[test]
    fn le_scadenze_di_oggi_si_trovano() {
        let mut memoria = MemoriaEsame::default();
        memoria.carte.push(carta_nuova());
        let mut futura = carta_nuova();
        futura.prossima = "2026-12-01".to_string();
        memoria.carte.push(futura);

        let scadute = memoria.scadute("2026-11-15");
        assert_eq!(scadute, vec![0]);
        // In una data precedente nessuna delle due e' ancora scaduta.
        assert!(memoria.scadute("2026-10-01").is_empty());
    }

    #[test]
    fn la_stessa_domanda_non_diventa_due_carte() {
        let mut memoria = MemoriaEsame::default();
        memoria.carte.push(carta_nuova());
        assert!(memoria.contiene("La riforma di Tiberio Gracco"));
        assert!(memoria.contiene("  La riforma di Tiberio Gracco  "));
        assert!(!memoria.contiene("Un'altra cosa"));
    }

    #[test]
    fn una_carta_senza_risposta_non_conta() {
        let mut carta = carta_nuova();
        carta.risposta = "   ".to_string();
        let mut memoria = MemoriaEsame::default();
        memoria.carte.push(carta);
        assert!(memoria.scadute("2027-01-01").is_empty());
    }
}
