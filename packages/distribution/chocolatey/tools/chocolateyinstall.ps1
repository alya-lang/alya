$ErrorActionPreference = 'Stop'

$packageName = 'alya'
$version     = '0.0.20'
$toolsDir    = "$(Split-Path -parent $MyInvocation.MyCommand.Definition)"

$url64        = "https://github.com/alya-lang/alya/releases/download/v$version/alya-v$version-x86_64-windows.zip"
$checksum64   = 'c14b83149e38eef88b9557350f79e7adc0f1470d0e42e707ab7a91885613371a'
$urlArm64      = "https://github.com/alya-lang/alya/releases/download/v$version/alya-v$version-arm64-windows.zip"
$checksumArm64 = '61a0b9ba3bca1beb389202cb4652d9aec6ef764b321c33e444dde50c949bafdc'
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
