# Impacchetta l'estensione FocusAnchor in un .xpi (uno zip con manifest e bootstrap).
$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.IO.Compression.FileSystem

$qui = Split-Path -Parent $MyInvocation.MyCommand.Path
$uscita = Join-Path $qui "focusanchor.xpi"
if (Test-Path $uscita) { Remove-Item $uscita -Force }

$zip = [System.IO.Compression.ZipFile]::Open($uscita, 'Create')
try {
    foreach ($nome in @('manifest.json', 'bootstrap.js')) {
        $null = [System.IO.Compression.ZipFileExtensions]::CreateEntryFromFile(
            $zip, (Join-Path $qui $nome), $nome, 'Optimal')
    }
} finally {
    $zip.Dispose()
}

"creato: $uscita ($((Get-Item $uscita).Length) byte)"
