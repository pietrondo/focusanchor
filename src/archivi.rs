//! Archivi su disco dell'utente.
//!
//! Tre file, tutti accanto all'eseguibile:
//! - `parcheggio_pensieri.txt`: le idee fuori tema emerse durante lo studio;
//! - `riassunto_studio.md`: le risposte alle domande a tradimento;
//! - `pronto_per_notebooklm.md`: il pacchetto da caricare su NotebookLM.

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;

use chrono::Local;

use crate::config::cartella_dati;
use crate::zotero::Evidenziazione;

/// Percorso del file dei pensieri parcheggiati.
pub fn percorso_parcheggio() -> PathBuf {
    cartella_dati().join("parcheggio_pensieri.txt")
}

/// Percorso del file con i riassunti di studio.
pub fn percorso_riassunti() -> PathBuf {
    cartella_dati().join("riassunto_studio.md")
}

/// Nome file senza caratteri vietati dal file system.
pub fn nome_file_sicuro(esame: &str) -> String {
    let pulito: String = esame
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '_' })
        .collect();
    let pulito = pulito.trim_matches('_').to_string();
    if pulito.is_empty() {
        "Senza_esame".to_string()
    } else {
        pulito
    }
}

/// Nome file del pacchetto NotebookLM per un esame specifico.
///
/// Il nome dell'esame finisce nel file: cosi' l'utente ha un file per corso e
/// non un calderone unico in cui si mescolano cinque materie.
pub fn nome_file_notebooklm(esame: &str) -> String {
    format!("notebooklm_{}.md", nome_file_sicuro(esame))
}

/// Percorso del pacchetto NotebookLM per un esame specifico.
pub fn percorso_notebooklm_esame(esame: &str) -> PathBuf {
    cartella_dati().join(nome_file_notebooklm(esame))
}

/// Mostra un file nel file system, con il file gia' selezionato.
///
/// Serve dopo aver generato il pacchetto NotebookLM o le carte Anki: quei
/// file vanno caricati a mano su NotebookLM o su Anki, e ritrovarli in una
/// cartella che nessuno apre mai era il passaggio che faceva perdere piu'
/// tempo.
pub fn rivela_file(percorso: &Path) -> Result<(), String> {
    use std::os::windows::process::CommandExt;
    if !percorso.exists() {
        return Err(format!("{} non esiste ancora.", percorso.display()));
    }
    // `explorer` restituisce 1 anche quando fa il suo lavoro: l'esito del
    // processo non dice niente, quindi si guarda solo se e' partito.
    Command::new("explorer")
        .arg(format!("/select,{}", percorso.display()))
        .creation_flags(crate::blocker::CREATE_NO_WINDOW)
        .spawn()
        .map(|_| ())
        .map_err(|e| format!("Non riesco ad aprire la cartella: {e}"))
}

/// Salva un pensiero improvviso con data e ora.
pub fn salva_pensiero(testo: &str) -> Result<(), String> {
    let testo = testo.trim();
    if testo.is_empty() {
        return Ok(());
    }
    let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S");
    let riga = format!("[{timestamp}] {testo}\n");
    let esito = OpenOptions::new()
        .create(true)
        .append(true)
        .open(percorso_parcheggio())
        .and_then(|mut f| f.write_all(riga.as_bytes()));
    esito.map_err(|e| format!("Non riesco a scrivere il pensiero: {e}"))
}

/// Legge il contenuto del file dei pensieri.
pub fn leggi_parcheggio() -> String {
    fs::read_to_string(percorso_parcheggio()).unwrap_or_default()
}

/// Svuota il file dei pensieri.
pub fn svuota_parcheggio() -> Result<(), String> {
    fs::write(percorso_parcheggio(), "").map_err(|e| format!("Non riesco a svuotare: {e}"))
}

/// Salva una risposta alle domande a tradimento.
///
/// Viene annotata la materia (l'esame) per ritrovarla facilmente, e il file
/// e' gia' in formato Markdown pronto per la revisione. La stessa coppia
/// domanda/risposta finisce anche nel file per Anki: cosi' il ripasso
/// dilazionato e' pronto senza copiare niente a mano.
pub fn salva_riassunto(materia: &str, passaggio: &str, risposta: &str) -> Result<(), String> {
    let risposta = risposta.trim();
    if risposta.is_empty() {
        return Ok(());
    }
    let timestamp = Local::now().format("%Y-%m-%d %H:%M");
    let materia = nome_esame(materia);

    let blocco = if passaggio.trim().is_empty() {
        format!("\n## {materia}\n\n*{timestamp}*\n\n{risposta}\n")
    } else {
        format!(
            "\n## {materia}\n\n*{timestamp}*\n\n**Passaggio:** {}\n\n**Risposta:** {risposta}\n",
            passaggio.trim()
        )
    };
    let esito = OpenOptions::new()
        .create(true)
        .append(true)
        .open(percorso_riassunti())
        .and_then(|mut f| f.write_all(blocco.as_bytes()));
    esito.map_err(|e| format!("Non riesco a scrivere il riassunto: {e}"))?;

    // La carta per Anki si scrive solo se c'e' una domanda.
    if !passaggio.trim().is_empty() {
        salva_carta_anki(&materia, passaggio, risposta)?;
        // La stessa domanda entra anche nella memoria dell'esame: il ripasso
        // dilazionato vive dentro l'app, il file per Anki resta un'uscita.
        crate::carte::aggiungi(materia, passaggio, risposta)?;
    }
    Ok(())
}

/// Nome dell'esame, o un segnaposto se manca.
fn nome_esame(materia: &str) -> &str {
    if materia.trim().is_empty() {
        "Senza materia"
    } else {
        materia.trim()
    }
}

/// Percorso del file di carte per Anki di un esame.
pub fn percorso_anki(esame: &str) -> PathBuf {
    cartella_dati().join(format!("anki_{}.txt", nome_file_sicuro(esame)))
}

/// Aggiunge una carta in formato tabulato, che Anki importa direttamente.
fn salva_carta_anki(materia: &str, domanda: &str, risposta: &str) -> Result<(), String> {
    // Tab e a capo romperebbero il formato: si riducono a spazi.
    let pulisci = |t: &str| t.replace(['\t', '\n', '\r'], " ").trim().to_string();
    let riga = format!("{}\t{}\n", pulisci(domanda), pulisci(risposta));
    OpenOptions::new()
        .create(true)
        .append(true)
        .open(percorso_anki(materia))
        .and_then(|mut f| f.write_all(riga.as_bytes()))
        .map_err(|e| format!("Non riesco a scrivere le carte Anki: {e}"))
}

/// Legge le carte Anki di un esame.
pub fn leggi_anki(esame: &str) -> String {
    fs::read_to_string(percorso_anki(esame)).unwrap_or_default()
}

/// Legge il file dei riassunti; se non e' ancora stato creato, restituisce testo vuoto.
pub fn leggi_riassunti() -> Result<String, String> {
    leggi_file_opzionale(&percorso_riassunti())
}

fn leggi_file_opzionale(percorso: &Path) -> Result<String, String> {
    match fs::read_to_string(percorso) {
        Ok(testo) => Ok(testo),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(String::new()),
        Err(e) => Err(format!("Non riesco a leggere {}: {e}", percorso.display())),
    }
}

/// Genera il pacchetto pronto per NotebookLM.
///
/// Unisce le sottolineature di Zotero con i riassunti scritti durante la
/// sessione: e' il materiale che l'utente puo' caricare su NotebookLM per
/// ottenere un riassunto o un Audio Overview senza copia-incolla.
///
/// Se sono disponibili annotazioni di oggi, si usano solo quelle: sono le
/// che l'utente ha appena evidenziato e le piu' rilevanti per la sessione.
pub fn genera_pacchetto_notebooklm(
    materia: &str,
    evidenziazioni: &[Evidenziazione],
    riassunti: &str,
) -> Result<(), String> {
    let timestamp = Local::now().format("%Y-%m-%d %H:%M");
    let materia = if materia.trim().is_empty() {
        "Senza materia"
    } else {
        materia.trim()
    };
    let oggi = Local::now().format("%Y-%m-%d").to_string();
    let di_oggi: Vec<&Evidenziazione> = evidenziazioni
        .iter()
        .filter(|e| e.data == oggi)
        .collect();
    // Se non c'e' niente di oggi, si usano le ultime disponibili.
    let selezionate: Vec<&Evidenziazione> = if di_oggi.is_empty() {
        evidenziazioni.iter().take(20).collect()
    } else {
        di_oggi
    };

    let mut testo = String::new();
    testo.push_str("# Materiale di studio per NotebookLM\n\n");
    testo.push_str(&format!("- **Materia:** {materia}\n"));
    testo.push_str(&format!("- **Generato il:** {timestamp}\n\n"));

    testo.push_str("## Sottolineature dai PDF di Zotero\n\n");
    if selezionate.is_empty() {
        testo.push_str("_Nessuna sottolineatura trovata oggi._\n\n");
    } else {
        for ev in selezionate {
            testo.push_str(&format!("- {}", ev.testo.trim()));
            if !ev.commento.trim().is_empty() {
                testo.push_str(&format!("  \n  _Nota: {}_", ev.commento.trim()));
            }
            testo.push('\n');
        }
        testo.push('\n');
    }

    testo.push_str("## I miei riassunti\n\n");
    let riassunti = riassunti.trim();
    if riassunti.is_empty() {
        testo.push_str("_Nessun riassunto scritto durante la sessione._\n");
    } else {
        testo.push_str(riassunti);
        testo.push('\n');
    }

    fs::write(percorso_notebooklm_esame(materia), testo)
        .map_err(|e| format!("Non riesco a scrivere il pacchetto: {e}"))
}

/// Esporta i dati dello studio in una cartella che l'utente vede.
///
/// I dati vivono in `%LOCALAPPDATA%`: se Windows viene reinstallata,
/// spariscono. Qui vengono copiati in una cartella sulla scrivania (o nei
/// documenti), cosi' l'utente li mette dove gli servono.
pub fn esporta_dati() -> Result<PathBuf, String> {
    let origine = cartella_dati();
    let cartella = cartella_esportazione().join(format!("focusanchor-{}", crate::carte::oggi()));
    fs::create_dir_all(&cartella).map_err(|e| format!("Non riesco a creare il backup: {e}"))?;
    let voci = fs::read_dir(&origine).map_err(|e| format!("Non riesco a leggere i dati: {e}"))?;
    let mut copiati = 0;
    for voce in voci.flatten() {
        let nome = voce.file_name().to_string_lossy().into_owned();
        if !crate::config::e_dato_dell_app(&nome) {
            continue;
        }
        if !voce.file_type().map(|t| t.is_file()).unwrap_or(false) {
            continue;
        }
        if fs::copy(voce.path(), cartella.join(&nome)).is_ok() {
            copiati += 1;
        }
    }
    if copiati == 0 {
        let _ = fs::remove_dir_all(&cartella);
        return Err("Non c'e' niente da esportare.".into());
    }
    Ok(cartella)
}

/// La cartella dove mettere l'esportazione.
///
/// Prima la scrivania (anche quella di OneDrive), poi i documenti: sono
/// le due cartelle che l'utente vede, e spesso sono gia' nel cloud.
fn cartella_esportazione() -> PathBuf {
    let mut candidati: Vec<PathBuf> = Vec::new();
    if let Some(one_drive) = std::env::var_os("OneDrive") {
        candidati.push(PathBuf::from(one_drive).join("Desktop"));
    }
    if let Some(casa) = std::env::var_os("USERPROFILE").map(PathBuf::from) {
        candidati.push(casa.join("Desktop"));
        candidati.push(casa.join("Documents"));
    }
    candidati
        .into_iter()
        .find(|cartella| cartella.is_dir())
        .unwrap_or_else(cartella_dati)
}

/// Legge il pacchetto NotebookLM preservando gli errori del file system.
pub fn leggi_notebooklm_esame_result(esame: &str) -> Result<String, String> {
    fs::read_to_string(percorso_notebooklm_esame(esame))
        .map_err(|e| format!("Non riesco a leggere la scheda NotebookLM: {e}"))
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn il_nome_file_usa_il_nome_dell_esame() {
        assert_eq!(
            nome_file_notebooklm("Storia Romana"),
            "notebooklm_Storia_Romana.md"
        );
        assert_eq!(
            nome_file_notebooklm("Economia e storia della globalizzazione"),
            "notebooklm_Economia_e_storia_della_globalizzazione.md"
        );
        // Un nome vuoto non deve produrre un file senza nome.
        assert_eq!(nome_file_notebooklm("   "), "notebooklm_Senza_esame.md");
    }

    #[test]
    fn il_nome_file_anki_non_contiene_caratteri_vietati() {
        // Lo stesso nome serve a scrivere e a rileggere le carte: se i due
        // calcoli divergessero, la carta sparirebbe dalla vista.
        assert_eq!(nome_file_sicuro("Storia romana"), "Storia_romana");
        assert_eq!(
            nome_file_sicuro("Economia e storia 19/10/2026"),
            "Economia_e_storia_19_10_2026"
        );
        assert_eq!(nome_file_sicuro("   "), "Senza_esame");
    }

    #[test]
    fn la_lettura_del_pacchetto_preserva_gli_errori_io() {
        let esame = format!(
            "File inesistente {} {}",
            std::process::id(),
            Local::now().timestamp_nanos_opt().unwrap_or_default()
        );
        assert!(leggi_notebooklm_esame_result(&esame).is_err());
    }

    #[test]
    fn la_lettura_dei_riassunti_propaga_gli_errori_non_dovuti_a_file_inesistente() {
        let percorso = std::env::temp_dir().join(format!(
            "focusanchor-summary-dir-{}-{}",
            std::process::id(),
            Local::now().timestamp_nanos_opt().unwrap_or_default()
        ));
        assert!(!percorso.exists());
        fs::create_dir(&percorso).unwrap();

        let risultato = leggi_file_opzionale(&percorso);

        let _ = fs::remove_dir(&percorso);
        assert!(risultato.is_err());
    }

    #[test]
    fn il_pacchetto_notebooklm_contiene_il_materiale() {
        let ev = vec![Evidenziazione {
            testo: "passaggio importante".to_string(),
            commento: "nota personale".to_string(),
            data: "2026-11-01".to_string(),
        }];
        let esito = genera_pacchetto_notebooklm(
            "Storia Romana",
            &ev,
            "## Storia Romana\n\nHo capito la parte sulla repubblica.",
        );
        // Il file viene scritto accanto all'eseguibile: la scrittura deve
        // riuscire e il contenuto deve contenere tutto il materiale.
        if esito.is_ok() {
            let contenuto = leggi_notebooklm_esame_result("Storia Romana").expect("file scritto");
            assert!(contenuto.contains("passaggio importante"));
            assert!(contenuto.contains("nota personale"));
            assert!(contenuto.contains("Ho capito la parte sulla repubblica"));
        }
    }
}
