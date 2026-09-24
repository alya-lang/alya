$ErrorActionPreference = 'Stop'

$packageName = 'alya'
$version     = '0.0.19'
$toolsDir    = "$(Split-Path -parent $MyInvocation.MyCommand.Definition)"

$url64        = "https://github.com/alya-lang/alya/releases/download/v$version/alya-v$version-x86_64-windows.zip"
$checksum64   = 'e1a70edba76dbc96ccb6c4960d19281ecc59a5a3bbacaa5f393979e18e2978b3'
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
