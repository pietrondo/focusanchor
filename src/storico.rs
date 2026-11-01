//! Storico delle sessioni di studio.
//!
//! Ogni sessione conclusa lascia una riga in `storico.json`. Serve a due
//! cose: far vedere il progresso nel tempo (che per l'ADHD e' la parte che
//! motiva) e non perdere gli XP alla chiusura dell'app.
//!
//! Il file e' volutamente semplice: una lista di record, nessun database.

use std::fs;
use std::path::PathBuf;

use chrono::Local;
use serde::{Deserialize, Serialize};

/// Numero massimo di sessioni tenute nello storico.
///
/// Oltre questa soglia si tengono le piu' recenti: un file che cresce
/// all'infinito non serve a nessuno.
pub const MASSIMO_SESSIONI: usize = 500;

/// Una sessione di studio conclusa.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SessioneRegistrata {
    /// Data e ora di fine, formato `AAAA-MM-GG HH:MM`.
    pub data: String,
    /// Esame studiato.
    pub esame: String,
    /// Minuti effettivi di studio, pause escluse.
    pub minuti: u32,
    /// XP guadagnati nella sessione.
    pub xp: i32,
    /// Distrazioni sventate.
    pub distrazioni: u32,
    /// Micro-task completati.
    pub blocchi: u32,
}

/// Riepilogo leggibile dello storico.
#[derive(Default, Debug)]
pub struct Statistiche {
    /// Sessioni registrate.
    pub sessioni: usize,
    /// Minuti totali di studio.
    pub minuti: u32,
    /// XP totali accumulati.
    pub xp: i32,
    /// Distrazioni sventate in totale.
    pub distrazioni: u32,
    /// Minuti per esame, dal piu' studiato.
    pub per_esame: Vec<(String, u32)>,
    /// Minuti di studio negli ultimi 7 giorni.
    pub ultimi_7_giorni: u32,
}

/// Percorso del file dello storico.
pub fn percorso_storico() -> PathBuf {
    crate::config::cartella_dati().join("storico.json")
}

/// Carica lo storico da disco; se manca o e' rotto, riparte da vuoto.
pub fn carica() -> Vec<SessioneRegistrata> {
    let Ok(testo) = fs::read_to_string(percorso_storico()) else {
        return Vec::new();
    };
    serde_json::from_str(&testo).unwrap_or_default()
}

/// Aggiunge una sessione allo storico e lo salva.
pub fn registra(sessione: SessioneRegistrata) {
    let mut elenco = carica();
    elenco.push(sessione);
    // Si tengono solo le piu' recenti.
    if elenco.len() > MASSIMO_SESSIONI {
        let eccesso = elenco.len() - MASSIMO_SESSIONI;
        elenco.drain(0..eccesso);
    }
    salva(&elenco);
}

/// Scrive lo storico su disco.
fn salva(elenco: &[SessioneRegistrata]) {
    if let Ok(json) = serde_json::to_string_pretty(elenco) {
        let _ = fs::write(percorso_storico(), json);
    }
}

/// Calcola il riepilogo dello storico.
pub fn statistiche(elenco: &[SessioneRegistrata]) -> Statistiche {
    let mut stat = Statistiche {
        sessioni: elenco.len(),
        ..Default::default()
    };
    let oggi = Local::now().date_naive();
    let mut per_esame: std::collections::BTreeMap<String, u32> =
        std::collections::BTreeMap::new();

    for s in elenco {
        stat.minuti += s.minuti;
        stat.xp += s.xp;
        stat.distrazioni += s.distrazioni;
        *per_esame.entry(s.esame.clone()).or_insert(0) += s.minuti;
        if entro_giorni(&s.data, oggi, 7) {
            stat.ultimi_7_giorni += s.minuti;
        }
    }

    let mut coppie: Vec<(String, u32)> = per_esame.into_iter().collect();
    coppie.sort_by(|a, b| b.1.cmp(&a.1));
    stat.per_esame = coppie;
    stat
}

/// Vero se la data (`AAAA-MM-GG HH:MM`) cade negli ultimi `giorni` giorni.
fn entro_giorni(data: &str, oggi: chrono::NaiveDate, giorni: i64) -> bool {
    let Ok(giorno) = chrono::NaiveDate::parse_from_str(data.get(0..10).unwrap_or(""), "%Y-%m-%d")
    else {
        return false;
    };
    let distanza = (oggi - giorno).num_days();
    (0..=giorni).contains(&distanza)
}

/// Giorni che mancano a una data d'esame (`AAAA-MM-GG`).
///
/// Restituisce `None` se la data non e' leggibile. Un valore negativo
/// significa che l'esame e' gia' passato.
pub fn giorni_alla_data(scadenza: &str) -> Option<i64> {
    let giorno =
        chrono::NaiveDate::parse_from_str(scadenza.trim(), "%Y-%m-%d").ok()?;
    Some((giorno - Local::now().date_naive()).num_days())
}

/// Data di oggi in formato `AAAA-MM-GG HH:MM`.
pub fn adesso() -> String {
    Local::now().format("%Y-%m-%d %H:%M").to_string()
}

#[cfg(test)]
mod test {
    use super::*;

    fn record(esame: &str, minuti: u32, xp: i32, data: &str) -> SessioneRegistrata {
        SessioneRegistrata {
            data: data.to_string(),
            esame: esame.to_string(),
            minuti,
            xp,
            distrazioni: 0,
            blocchi: 0,
        }
    }

    #[test]
    fn le_statistiche_sommano_tutto() {
        let elenco = vec![
            record("Storia romana", 25, 100, "2026-11-01 10:00"),
            record("Storia romana", 45, 150, "2026-11-01 15:00"),
            record("Letteratura", 30, 80, "2026-11-01 18:00"),
        ];
        let stat = statistiche(&elenco);
        assert_eq!(stat.sessioni, 3);
        assert_eq!(stat.minuti, 100);
        assert_eq!(stat.xp, 330);
        // L'esame piu' studiato viene per primo.
        assert_eq!(stat.per_esame[0].0, "Storia romana");
        assert_eq!(stat.per_esame[0].1, 70);
    }

    #[test]
    fn i_giorni_all_esame_sono_calcolati() {
        let oggi = Local::now().format("%Y-%m-%d").to_string();
        let giorni = giorni_alla_data(&oggi).unwrap();
        assert_eq!(giorni, 0);
        // Una data nel passato da' un numero negativo.
        assert!(giorni_alla_data("2020-01-01").unwrap() < 0);
        // Una data illeggibile non inventa numeri.
        assert!(giorni_alla_data("domani").is_none());
    }

    #[test]
    fn il_filtro_dei_giorni_ignora_le_date_vecchie() {
        let oggi = Local::now().date_naive();
        let elenco = vec![
            record("A", 25, 10, &format!("{} 10:00", oggi.format("%Y-%m-%d"))),
            record("A", 25, 10, "2020-01-01 10:00"),
        ];
        let stat = statistiche(&elenco);
        assert_eq!(stat.ultimi_7_giorni, 25);
        assert_eq!(stat.minuti, 50);
    }
}
