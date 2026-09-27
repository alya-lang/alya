$ErrorActionPreference = 'Stop'

$packageName = 'alya'
$version     = '0.0.19'
$toolsDir    = "$(Split-Path -parent $MyInvocation.MyCommand.Definition)"

$url64        = "https://github.com/alya-lang/alya/releases/download/v$version/alya-v$version-x86_64-windows.zip"
$checksum64   = '81711018a0d5974d29e68cc09d7f265e4099f2914017fe78c83d146f70022364'
$urlArm64      = "https://github.com/alya-lang/alya/releases/download/v$version/alya-v$version-arm64-windows.zip"
$checksumArm64 = '56eac0afecf5f87e84d7d5b91d16e78ffbd24277b01db39f268631b75be7b7ab'
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
