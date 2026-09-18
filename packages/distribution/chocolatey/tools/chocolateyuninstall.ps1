$ErrorActionPreference = 'Stop'

$packageName = 'alya'
$toolsDir    = "$(Split-Path -parent $MyInvocation.MyCommand.Definition)"

# Remove extracted executable and artifacts while preserving installer scripts
Get-ChildItem -Path $toolsDir -Exclude '*.ps1', '*.txt' | Remove-Item -Force -Recurse -ErrorAction SilentlyContinue

Write-Host "Alya compiler ($packageName) uninstalled successfully."
