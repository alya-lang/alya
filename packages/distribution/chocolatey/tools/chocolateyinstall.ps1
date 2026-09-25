$ErrorActionPreference = 'Stop'

$packageName = 'alya'
$version     = '0.0.19'
$toolsDir    = "$(Split-Path -parent $MyInvocation.MyCommand.Definition)"

$url64        = "https://github.com/alya-lang/alya/releases/download/v$version/alya-v$version-x86_64-windows.zip"
$checksum64   = '351c68fc0a1278d2ac5faa5a5e8df9226e51a596a8296360b2728b6b550f7d41'
$urlArm64      = "https://github.com/alya-lang/alya/releases/download/v$version/alya-v$version-arm64-windows.zip"
$checksumArm64 = '387e4df860895be0c0046ec7619c7394d0d9587fe498958d7c42001aad261455'
$checksumType  = 'sha256'

$packageArgs = @{
  packageName   = $packageName
  unzipLocation = $toolsDir
}

# Install-ChocolateyZipPackage has no ARM64 URL slot, so select the native
# archive explicitly instead of relying on OS-width detection.
if ($env:PROCESSOR_ARCHITECTURE -eq 'ARM64') {
  $packageArgs.url          = $urlArm64
  $packageArgs.checksum     = $checksumArm64
  $packageArgs.checksumType = $checksumType
} else {
  $packageArgs.url64          = $url64
  $packageArgs.checksum64     = $checksum64
  $packageArgs.checksumType64 = $checksumType
}

Install-ChocolateyZipPackage @packageArgs

# Move extracted files to toolsDir root if nested inside subfolder (x64 or ARM64)
foreach ($nestedName in @("alya-v$version-x86_64-windows", "alya-v$version-arm64-windows")) {
    $nestedDir = Join-Path $toolsDir $nestedName
    if (Test-Path $nestedDir) {
        Get-ChildItem -Path $nestedDir | ForEach-Object {
            Move-Item -Path $_.FullName -Destination $toolsDir -Force
        }
        Remove-Item -Path $nestedDir -Force -Recurse
    }
}

Write-Host "Alya compiler ($packageName) installed successfully to $toolsDir"
