$ErrorActionPreference = 'Stop'

$packageName = 'alya'
$version     = '0.0.18'
$toolsDir    = "$(Split-Path -parent $MyInvocation.MyCommand.Definition)"

$url64        = "https://github.com/alya-lang/alya/releases/download/v$version/alya-v$version-x86_64-windows.zip"
$checksum64   = '11cffc205cb2caca48de716de73f48c98c33917b2da1f3da8b8585422290667e'
$checksumType = 'sha256'

$packageArgs = @{
  packageName   = $packageName
  unzipLocation = $toolsDir
  url64         = $url64
  checksum64    = $checksum64
  checksumType64= $checksumType
}

Install-ChocolateyZipPackage @packageArgs

# Move extracted files to toolsDir root if nested inside subfolder
$nestedDir = Join-Path $toolsDir "alya-v$version-x86_64-windows"
if (Test-Path $nestedDir) {
    Get-ChildItem -Path $nestedDir | ForEach-Object {
        Move-Item -Path $_.FullName -Destination $toolsDir -Force
    }
    Remove-Item -Path $nestedDir -Force -Recurse
}

Write-Host "Alya compiler ($packageName) installed successfully to $toolsDir"
