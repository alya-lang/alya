#Requires -Version 5.1
# Provisions the Windows ARM64 C toolchain (LLVM-MinGW) into ~/.alya/toolchain
# for CI runners that lack a system C toolchain (windows-11-arm).
$ErrorActionPreference = "Stop"

$tcDir = "$env:USERPROFILE/.alya/toolchain"
New-Item -ItemType Directory -Force -Path $tcDir | Out-Null
$zip = "$env:RUNNER_TEMP/toolchain-arm64.zip"
[Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12
try {
    Invoke-WebRequest -Uri "https://github.com/alya-lang/toolchain/releases/latest/download/alya-toolchain-windows-arm64.zip" -OutFile $zip -UseBasicParsing
} catch {
    Write-Host "Latest toolchain download failed, falling back to v1.0.0"
    Invoke-WebRequest -Uri "https://github.com/alya-lang/toolchain/releases/download/v1.0.0/alya-toolchain-windows-arm64.zip" -OutFile $zip -UseBasicParsing
}
Expand-Archive -Path $zip -DestinationPath $tcDir -Force
Add-Content -Path $env:GITHUB_PATH -Value "$tcDir/bin"
