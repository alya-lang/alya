#Requires -Version 5.1
# Packages Windows alya release archives (both `alya` and `alya-lsp`):
#   1. Standard lightweight package (binaries + README + LICENSE + SHA-256)
#   2. Standalone offline package (with pre-bundled C toolchain)
# Usage: package-windows-release.ps1 -Version <v> -Platform <p> -Target <t> -Bin <b> -Tc <toolchain-zip>
[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)][string]$Version,
    [Parameter(Mandatory = $true)][string]$Platform,
    [Parameter(Mandatory = $true)][string]$Target,
    [Parameter(Mandatory = $true)][string]$Bin,
    [Parameter(Mandatory = $true)][string]$Tc
)

$ErrorActionPreference = "Stop"

# 1. Standard lightweight package
$PackageName = "alya-$Version-$Platform"
New-Item -ItemType Directory -Force -Path $PackageName | Out-Null
Copy-Item "target/$Target/release/$Bin" -Destination $PackageName
# Standalone LSP server ships in the same archive so editors fetch one
# file and use `alya-lsp` for editing plus `alya` for run/build/test.
$LspBin = if ($Bin -eq "alya.exe") { "alya-lsp.exe" } else { $Bin -replace 'alya(\.exe)?$', 'alya-lsp$1' }
$LspPath = "target/$Target/release/$LspBin"
if (-not (Test-Path $LspPath)) {
  # Manifests (scoop, winget) promise this file: never ship without it.
  throw "Missing standalone LSP binary: $LspPath"
}
Copy-Item $LspPath -Destination $PackageName
Copy-Item README.md, LICENSE -Destination $PackageName
Compress-Archive -Path $PackageName -DestinationPath "$PackageName.zip"
$hash = (Get-FileHash -Algorithm SHA256 "$PackageName.zip").Hash.ToLower()
"$hash  $PackageName.zip" | Out-File -FilePath "$PackageName.zip.sha256" -Encoding ascii

# 2. Standalone offline package (with pre-bundled toolchain)
$StandaloneName = "alya-$Version-$Platform-standalone"
New-Item -ItemType Directory -Force -Path $StandaloneName | Out-Null
Copy-Item "target/$Target/release/$Bin" -Destination $StandaloneName
# $LspPath existence already enforced above (throw); unconditional here.
Copy-Item $LspPath -Destination $StandaloneName
Copy-Item README.md, LICENSE -Destination $StandaloneName

# Download and bundle minimal toolchain (tracking latest release, with fallback)
$tcUrl = "https://github.com/alya-lang/toolchain/releases/latest/download/$Tc"
$tcZip = Join-Path $StandaloneName "toolchain.zip"
$tcDir = Join-Path $StandaloneName "toolchain"
try {
    [Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12
    try {
        Invoke-WebRequest -Uri $tcUrl -OutFile $tcZip -UseBasicParsing
    } catch {
        Write-Host "Latest toolchain download failed, falling back to v1.0.0: $_"
        Invoke-WebRequest -Uri "https://github.com/alya-lang/toolchain/releases/download/v1.0.0/$Tc" -OutFile $tcZip -UseBasicParsing
    }
    Expand-Archive -Path $tcZip -DestinationPath $tcDir -Force
    Remove-Item -Force $tcZip
} catch {
    Write-Host "Warning: Could not fetch pre-bundled toolchain for standalone package: $_"
}

Compress-Archive -Path $StandaloneName -DestinationPath "$StandaloneName.zip"
$hashStandalone = (Get-FileHash -Algorithm SHA256 "$StandaloneName.zip").Hash.ToLower()
"$hashStandalone  $StandaloneName.zip" | Out-File -FilePath "$StandaloneName.zip.sha256" -Encoding ascii
