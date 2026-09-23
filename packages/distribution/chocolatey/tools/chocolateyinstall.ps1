$ErrorActionPreference = 'Stop'

$packageName = 'alya'
$version     = '0.0.19'
$toolsDir    = "$(Split-Path -parent $MyInvocation.MyCommand.Definition)"

$url64        = "https://github.com/alya-lang/alya/releases/download/v$version/alya-v$version-x86_64-windows.zip"
$checksum64   = '9154a95792175d53cafb0ca6b7b8c36690a35890322a7a53832b30b381b2f123'
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
