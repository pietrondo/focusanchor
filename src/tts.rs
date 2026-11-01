//! Script PowerShell per la sintesi vocale nativa di Windows.
//!
//! Usa `System.Speech.Synthesis.SpeechSynthesizer`, disponibile in Windows
//! PowerShell 5.1: nessuna libreria C++ da compilare, quindi `cargo build`
//! non puo' fallire per dipendenze mancanti.

/// Costruisce lo script che legge il testo da stdin e lo pronuncia.
///
/// La velocita' (`Rate`) e' regolabile: ascoltare a velocita' sostenuta
/// combatte la noia tipica dell'ADHD.
///
/// Il lettore e' esplicito su UTF-8: `[Console]::In` userebbe la codepage
/// ANSI di Windows e trasformerebbe gli accenti italiani in spazzatura.
pub fn script_tts(rate: i32) -> String {
    format!(
        r#"
$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Speech
$voce = New-Object System.Speech.Synthesis.SpeechSynthesizer
$voce.Rate = {rate}
$voce.Volume = 100
$flusso = [Console]::OpenStandardInput()
$lettore = New-Object System.IO.StreamReader($flusso, [Text.Encoding]::UTF8)
$testo = $lettore.ReadToEnd()
if ($testo.Trim().Length -gt 0) {{ $voce.Speak($testo) }}
$voce.Dispose()
"#
    )
}

#[cfg(test)]
mod test {
    use super::script_tts;

    #[test]
    fn lo_script_contiene_la_velocita_richiesta() {
        assert!(script_tts(3).contains("$voce.Rate = 3"));
        assert!(script_tts(5).contains("$voce.Rate = 5"));
    }

    #[test]
    fn lo_script_usa_la_lettura_utf8() {
        let script = script_tts(1);
        assert!(script.contains("[Text.Encoding]::UTF8"));
        assert!(script.contains("OpenStandardInput"));
    }
}
