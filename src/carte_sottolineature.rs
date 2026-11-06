//! Carte nate dalle sottolineature di Zotero.
//!
//! Servono a non avere il ripasso vuoto prima della prima sessione: le
//! sottolineature sono materiale gia' scelto da te, e chiederti di spiegarle
//! e' il modo piu' diretto di ripassarle.

use crate::carte::{self, Carta, MemoriaEsame};

/// Crea una carta per ogni sottolineatura dell'esame.
///
/// La domanda e' solo l'apertura del passaggio, cosi' non contiene la
/// risposta: il resto lo devi dire tu, poi confronti.
pub fn genera(
    esame: &str,
    evidenze: &[crate::zotero::Evidenziazione],
) -> Result<(MemoriaEsame, usize), String> {
    let mut memoria = carte::carica(esame);
    let oggi = carte::oggi();
    let mut aggiunte = 0;

    for evidenza in evidenze {
        let testo = evidenza.testo.trim();
        // Troppo corta: l'apertura sarebbe tutta la risposta.
        if testo.chars().count() < 40 || memoria.contiene(testo) {
            continue;
        }
        let risposta = if evidenza.commento.trim().is_empty() {
            testo.to_string()
        } else {
            format!("{}\n\nNota tua: {}", testo, evidenza.commento.trim())
        };
        memoria.carte.push(Carta {
            domanda: format!(
                "Spiega con parole tue questo passaggio: «{}…»",
                apertura(testo, 12)
            ),
            risposta,
            prossima: oggi.clone(),
            intervallo: 0,
            facilita: 2.5,
            serie: 0,
        });
        aggiunte += 1;
    }

    if aggiunte > 0 {
        carte::salva(esame, &memoria)?;
    }
    Ok((memoria, aggiunte))
}

/// Prime `parole` parole di un testo.
fn apertura(testo: &str, parole: usize) -> String {
    testo
        .split_whitespace()
        .take(parole)
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn l_apertura_non_mostra_tutta_la_risposta() {
        let testo =
            "La riforma agraria di Tiberio Gracco prevedeva la ridistribuzione dell'agro pubblico";
        let cue = apertura(testo, 5);
        assert_eq!(cue, "La riforma agraria di Tiberio");
        assert!(testo.len() > cue.len());
        // Testo piu' corto delle parole richieste: non inventa nulla.
        assert_eq!(apertura("Due parole", 5), "Due parole");
        // Spazi doppi e a capo non devono produrre buchi.
        assert_eq!(apertura("uno  due\ntre", 3), "uno due tre");
    }
}
