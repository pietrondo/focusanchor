# Impacchetta l'estensione FocusAnchor in un .xpi (uno zip con manifest e codice).
$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.IO.Compression.FileSystem

$qui = Split-Path -Parent $MyInvocation.MyCommand.Path
$uscita = Join-Path $qui "focusanchor.xpi"
if (Test-Path $uscita) { Remove-Item $uscita -Force }

# Tutto quello che deve stare dentro lo zip, con il percorso relativo.
$dentro = @(
    'manifest.json',
    'bootstrap.js',
    'locale/it-IT/focusanchor.ftl',
    'locale/en-US/focusanchor.ftl',
    'chrome/content/icons/focusanchor.svg'
)

$zip = [System.IO.Compression.ZipFile]::Open($uscita, 'Create')
try {
    foreach ($relativo in $dentro) {
        $sorgente = Join-Path $qui $relativo
        if (-not (Test-Path $sorgente)) { throw "manca $relativo" }
        $null = [System.IO.Compression.ZipFileExtensions]::CreateEntryFromFile(
            $zip, $sorgente, $relativo, 'Optimal')
    }
} finally {
    $zip.Dispose()
}

"creato: $uscita ($((Get-Item $uscita).Length) byte)"
