//! Appelli: le date degli esami, il conto alla rovescia e il piano di studio.
//!
//! Le date si scrivono come sul portale dell'ateneo (`GG/MM/AAAA`) o in
//! formato ISO (`AAAA-MM-GG`): `storico::parse_data` le legge allo stesso
//! modo, cosi' non serve ricordare quale dei due si usa.

use std::collections::BTreeMap;

use crate::config::Configurazione;
use crate::storico::{self, SessioneRegistrata};
use crate::zotero::Collezione;

/// Un esame in vista: quando scade e quanto ci hai gia' studiato.
#[derive(Clone, Debug)]
pub struct Appello {
    /// Nome dell'esame.
    pub nome: String,
    /// Giorni che mancano. `None` se la data manca o non e' leggibile.
    pub giorni: Option<i64>,
    /// Minuti gia' studiati su questo esame.
    pub minuti: u32,
    /// Crediti formativi dell'esame.
    pub crediti: u32,
}

/// Il piano di studio di un esame: quanto resta e come dividerlo.
///
/// E' il ragionamento a ritroso che funziona: 40 giorni non sono 40 giorni
/// di studio, sono 40 occasioni per 20-30 minuti. Il piano dice quanti.
#[derive(Clone, Copy, Debug, Default)]
pub struct Piano {
    /// Documenti dell'esame, se Zotero e' caricato.
    pub documenti: Option<usize>,
    /// Documenti su cui hai gia' lavorato.
    pub letti: usize,
    /// Minuti che mancano per finire il materiale.
    pub minuti_rimanenti: Option<u32>,
    /// Minuti da studiare al giorno per arrivare all'appello.
    pub minuti_al_giorno: Option<u32>,
}

/// Elenco degli appelli: prima i piu' vicini, in fondo quelli gia' passati.
///
/// L'ordine e' quello che serve per decidere: gli appelli che scadono
/// vengono prima, cosi' il primo e' quello urgente.
pub fn elenco(config: &Configurazione, storico: &[SessioneRegistrata]) -> Vec<Appello> {
    let statistiche = storico::statistiche(storico);
    let mut appelli: Vec<Appello> = config
        .esami
        .iter()
        .map(|(nome, info)| Appello {
            nome: nome.clone(),
            giorni: storico::giorni_alla_data(&info.data_esame),
            minuti: statistiche
                .per_esame
                .iter()
                .find(|(esame, _)| esame == nome)
                .map(|(_, minuti)| *minuti)
                .unwrap_or(0),
            crediti: info.crediti,
        })
        .collect();
    // Gli appelli ancora da venire vengono per primi, dal piu' vicino.
    // Quelli passati o senza data leggibile stanno in fondo: non servono a
    // decidere cosa studiare adesso.
    appelli.sort_by_key(|appello| match appello.giorni {
        Some(giorni) if giorni >= 0 => giorni,
        _ => i64::MAX,
    });
    appelli
}

/// L'appello piu' vicino e gli esami che cadono lo stesso giorno.
///
/// Piu' esami sullo stesso appello sono la norma a dicembre: mostrare solo il
/// primo nome (spesso il piu' lungo di Zotero) nasconderebbe gli altri.
pub fn prossimo(config: &Configurazione) -> Option<(i64, Vec<String>)> {
    let mut minimo: Option<i64> = None;
    let mut nomi: Vec<String> = Vec::new();
    for (nome, info) in &config.esami {
        let Some(giorni) = storico::giorni_alla_data(&info.data_esame) else {
            continue;
        };
        if giorni < 0 {
            continue;
        }
        match minimo {
            None => {
                minimo = Some(giorni);
                nomi.push(nome.clone());
            }
            Some(solo) if giorni < solo => {
                minimo = Some(giorni);
                nomi.clear();
                nomi.push(nome.clone());
            }
            Some(solo) if giorni == solo => nomi.push(nome.clone()),
            _ => {}
        }
    }
    minimo.map(|giorni| (giorni, nomi))
}

/// Il conto alla rovescia dell'esame che stai studiando.
///
/// Durante la sessione conta l'appello di quell'esame, non quello di un
/// altro: altrimenti il conto parla di un esame che non stai guardando.
/// Se l'esame non ha una data, si cade sul piu' vicino.
pub fn conto_di(
    config: &Configurazione,
    esame: &str,
) -> Option<(i64, Vec<String>)> {
    if esame.is_empty() || esame == "Senza esame" {
        return prossimo(config);
    }
    let giorni = storico::giorni_alla_data(&config.esami.get(esame)?.data_esame);
    match giorni {
        Some(giorni) if giorni >= 0 => Some((giorni, vec![esame.to_string()])),
        // Senza data leggibile si cade sul piu' vicino: il `?` qui
        // restituirebbe `None` e l'overlay non mostrerebbe alcun conto.
        _ => prossimo(config),
    }
}

/// Riga leggibile del conto alla rovescia: `📅 Storia romana: fra 34 giorni`.
pub fn testo_conto(giorni: i64, esami: &[String]) -> String {
    let quando = match giorni {
        0 => "oggi".to_string(),
        1 => "domani".to_string(),
        n => format!("fra {n} giorni"),
    };
    match esami {
        [] => format!("📅 Appello {quando}"),
        [unico] => format!("📅 {}: {quando}", crate::ui::accorcia(unico, 30)),
        molti => format!("📅 {} esami {quando}", molti.len()),
    }
}

/// Minuti studiati su un esame negli ultimi `giorni` giorni.
///
/// Il ritmo di un esame e' diverso da quello globale: una settimana di
/// Storia Romana non dice niente su Letteratura.
pub fn minuti_recenti(
    storico: &[SessioneRegistrata],
    esame: &str,
    giorni: i64,
) -> u32 {
    let oggi = chrono::Local::now().date_naive();
    storico
        .iter()
        .filter(|s| s.esame == esame)
        .filter(|s| {
            let Ok(giorno) =
                chrono::NaiveDate::parse_from_str(s.data.get(0..10).unwrap_or(""), "%Y-%m-%d")
            else {
                return false;
            };
            let distanza = (oggi - giorno).num_days();
            (0..=giorni).contains(&distanza)
        })
        .map(|s| s.minuti)
        .sum()
}

/// Ritmo giornaliero di un esame: minuti al giorno negli ultimi sette giorni.
pub fn ritmo_esame(storico: &[SessioneRegistrata], esame: &str) -> u32 {
    minuti_recenti(storico, esame, 7) / 7
}

/// Secondi passati su ogni documento, raggruppati per esame.
///
/// Serve a stimare quanto costa davvero un documento: e' la misura del tuo
/// ritmo, non una stima generica.
pub fn secondi_per_documento(
    storico: &[SessioneRegistrata],
) -> BTreeMap<String, BTreeMap<String, u32>> {
    let mut per_esame: BTreeMap<String, BTreeMap<String, u32>> = BTreeMap::new();
    for sessione in storico {
        let documenti = per_esame.entry(sessione.esame.clone()).or_default();
        for (titolo, secondi) in &sessione.tempo_documenti {
            *documenti.entry(titolo.clone()).or_insert(0) += secondi;
        }
    }
    per_esame
}

/// Piano di studio di un esame.
///
/// I minuti per documento vengono da quello che hai gia' letto: e' una
/// media del tuo materiale, non una media di tutti gli studenti.
pub fn piano(
    esame: &str,
    secondi: &BTreeMap<String, BTreeMap<String, u32>>,
    collezioni: &[Collezione],
    giorni: Option<i64>,
) -> Piano {
    let documenti_esame = secondi.get(esame);
    let letti = documenti_esame.map(|d| d.len()).unwrap_or(0);
    let secondi_totali: u32 = documenti_esame
        .map(|d| d.values().sum())
        .unwrap_or(0);

    let totale = collezioni
        .iter()
        .find(|c| c.nome.eq_ignore_ascii_case(esame))
        .map(|c| c.documenti);
    let Some(totale) = totale else {
        return Piano {
            documenti: None,
            letti,
            minuti_rimanenti: None,
            minuti_al_giorno: None,
        };
    };

    let mut piano = Piano {
        documenti: Some(totale),
        letti,
        minuti_rimanenti: None,
        minuti_al_giorno: None,
    };
    if letti == 0 || secondi_totali == 0 {
        // Senza una base di misure non si stima niente: si dice solo quanti
        // documenti restano.
        return piano;
    }
    // La media si calcola in secondi: un documento di 40 secondi darebbe
    // zero minuti e il piano sparirebbe.
    let media = secondi_totali as f64 / letti as f64;
    let rimanenti = (totale.saturating_sub(letti) as f64 * media / 60.0).round() as u32;
    piano.minuti_rimanenti = Some(rimanenti);
    if let Some(giorni) = giorni {
        if giorni > 0 && rimanenti > 0 {
            piano.minuti_al_giorno = Some(rimanenti.div_ceil(giorni as u32));
        }
    }
    piano
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::config::InfoEsame;

    fn config_con(esami: &[(&str, &str)]) -> Configurazione {
        let mut config = Configurazione::default();
        for (nome, data) in esami {
            config.esami.insert(
                nome.to_string(),
                InfoEsame {
                    data_esame: data.to_string(),
                    ..Default::default()
                },
            );
        }
        config
    }

    fn giorni_futuri(giorni: i64) -> String {
        (chrono::Local::now().date_naive() + chrono::Duration::days(giorni))
            .format("%Y-%m-%d")
            .to_string()
    }

    fn collezione(nome: &str, documenti: usize) -> Collezione {
        Collezione {
            id: 1,
            nome: nome.to_string(),
            documenti,
            sottocollezioni: 0,
        }
    }

    #[test]
    fn gli_appelli_vengono_ordinati_per_scadenza() {
        let config = config_con(&[
            ("Lontano", &giorni_futuri(30)),
            ("Vicino", &giorni_futuri(5)),
        ]);

        let appelli = elenco(&config, &[]);

        assert_eq!(appelli[0].nome, "Vicino");
        assert_eq!(appelli[0].giorni, Some(5));
        assert_eq!(appelli[1].nome, "Lontano");
    }

    #[test]
    fn un_appello_passato_sta_in_fondo_alla_lista() {
        let config = config_con(&[("Passato", "2020-01-01"), ("Futuro", &giorni_futuri(3))]);

        let appelli = elenco(&config, &[]);

        assert_eq!(appelli[0].nome, "Futuro");
        assert_eq!(appelli[1].nome, "Passato");
    }

    #[test]
    fn la_data_si_legge_in_entrambi_i_formati() {
        let config = config_con(&[("Italiano", "16/12/2026"), ("Iso", "2026-12-16")]);

        let appelli = elenco(&config, &[]);

        let italiano = appelli.iter().find(|a| a.nome == "Italiano").unwrap();
        let iso = appelli.iter().find(|a| a.nome == "Iso").unwrap();
        assert_eq!(italiano.giorni, iso.giorni);
        assert!(italiano.giorni.is_some());
    }

    #[test]
    fn piu_esami_lo_stesso_giorno_contano_insieme() {
        let data = giorni_futuri(20);
        let config = config_con(&[("Storia romana", &data), ("Letteratura", &data)]);

        let (giorni, nomi) = prossimo(&config).expect("appello");

        assert_eq!(giorni, 20);
        assert_eq!(nomi.len(), 2);
    }

    #[test]
    fn un_appello_passato_non_e_il_prossimo() {
        let config = config_con(&[("Vecchio", "2020-01-01"), ("Nuovo", &giorni_futuri(9))]);

        let (giorni, nomi) = prossimo(&config).expect("appello");

        assert_eq!(giorni, 9);
        assert_eq!(nomi, vec!["Nuovo"]);
    }

    #[test]
    fn il_ritmo_e_il_piano_escono_dai_dati_reali() {
        let oggi = chrono::Local::now().date_naive();
        let mut sessione = SessioneRegistrata {
            data: (oggi - chrono::Duration::days(1)).format("%Y-%m-%d 10:00").to_string(),
            esame: "Storia romana".into(),
            minuti: 70,
            xp: 0,
            distrazioni: 0,
            blocchi: 0,
            tempo_documenti: Default::default(),
        };
        // Due documenti da 35 minuti: e' la base di misure del piano.
        sessione.tempo_documenti.insert("Manuale".into(), 35 * 60);
        sessione.tempo_documenti.insert("Riassunti".into(), 35 * 60);
        let storico = vec![sessione];

        // Il ritmo e' quello di questo esame, non quello globale.
        assert_eq!(ritmo_esame(&storico, "Storia romana"), 10);
        assert_eq!(ritmo_esame(&storico, "Altro esame"), 0);

        let secondi = secondi_per_documento(&storico);
        let collezioni = vec![collezione("Storia romana", 4)];
        let piano = piano("Storia romana", &secondi, &collezioni, Some(14));

        // 2 documenti letti su 4, 35 minuti ciascuno: 70 minuti restanti,
        // 5 minuti al giorno per 14 giorni.
        assert_eq!(piano.letti, 2);
        assert_eq!(piano.minuti_rimanenti, Some(70));
        assert_eq!(piano.minuti_al_giorno, Some(5));
    }

    #[test]
    fn il_conto_riguarda_l_esame_che_stai_studiando() {
        let config = config_con(&[
            ("Vicino", &giorni_futuri(5)),
            ("Quello che studio", &giorni_futuri(40)),
        ]);

        // Durante la sessione conta l'esame aperto, non il piu' vicino.
        let (giorni, nomi) = conto_di(&config, "Quello che studio").expect("conto");

        assert_eq!(giorni, 40);
        assert_eq!(nomi, vec!["Quello che studio"]);
    }

    #[test]
    fn un_esame_senza_data_cade_sul_piu_vicino() {
        let config = config_con(&[("Vicino", &giorni_futuri(5)), ("Senza data", "")]);

        let (giorni, nomi) = conto_di(&config, "Senza data").expect("conto");

        assert_eq!(giorni, 5);
        assert_eq!(nomi, vec!["Vicino"]);
    }

    #[test]
    fn il_piano_dividi_il_lavoro_sui_giorni_rimasti() {
        let mut sessione = SessioneRegistrata {
            data: "2026-11-01 10:00".into(),
            esame: "Storia romana".into(),
            minuti: 60,
            xp: 0,
            distrazioni: 0,
            blocchi: 0,
            tempo_documenti: Default::default(),
        };
        // Due documenti da 30 minuti l'uno.
        sessione
            .tempo_documenti
            .insert("Manuale".into(), 30 * 60);
        sessione
            .tempo_documenti
            .insert("Riassunti".into(), 30 * 60);
        let secondi = secondi_per_documento(&[sessione]);
        let collezioni = vec![collezione("Storia romana", 6)];

        // 4 documenti da finire, 30 minuti l'uno: 120 minuti in 10 giorni.
        let piano = piano("Storia romana", &secondi, &collezioni, Some(10));

        assert_eq!(piano.documenti, Some(6));
        assert_eq!(piano.letti, 2);
        assert_eq!(piano.minuti_rimanenti, Some(120));
        assert_eq!(piano.minuti_al_giorno, Some(12));
    }

    #[test]
    fn senza_una_base_di_misure_il_piano_non_stima_niente() {
        let secondi = secondi_per_documento(&[]);
        let collezioni = vec![collezione("Storia romana", 6)];

        let piano = piano("Storia romana", &secondi, &collezioni, Some(10));

        assert_eq!(piano.documenti, Some(6));
        assert_eq!(piano.letti, 0);
        assert_eq!(piano.minuti_rimanenti, None);
        assert_eq!(piano.minuti_al_giorno, None);
    }
}
