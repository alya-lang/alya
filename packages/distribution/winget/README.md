# WinGet Manifest for Alya Compiler (`alyac`)

Official WinGet manifest definition for installing the Alya programming language compiler via Windows Package Manager.

## Testing Locally

### 1. Validate the manifest
If you have `winget` installed:
```powershell
winget validate --manifest alyac.yaml
```

### 2. Test installation from local manifest
```powershell
winget install --manifest alyac.yaml
```

### 3. Verify
```powershell
alyac --version
alyac toolchain status
```

## Submitting to `microsoft/winget-pkgs`

You can submit new releases to the official Windows Package Manager repository:

### Method A: Automated via `wingetcreate`
```powershell
wingetcreate new https://github.com/alya-lang/alya/releases/download/v0.0.17/alyac-v0.0.17-x86_64-windows.zip
```

### Method B: Manual PR
Copy `alyac.yaml` to the community repository under:
`manifests/a/Alya/alyac/0.0.17/Alya.alyac.yaml`
and submit a Pull Request to [microsoft/winget-pkgs](https://github.com/microsoft/winget-pkgs).
