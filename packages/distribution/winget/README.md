# WinGet Manifest for Alya Compiler (`alya`)

Official WinGet manifest definition for installing the Alya programming language compiler via Windows Package Manager.

## Testing Locally

### 1. Validate the manifest
If you have `winget` installed:
```powershell
winget validate --manifest alya.yaml
```

### 2. Test installation from local manifest
```powershell
winget install --manifest alya.yaml
```

### 3. Verify
```powershell
alya --version
alya toolchain status
```

## Submitting to `microsoft/winget-pkgs`

You can submit new releases to the official Windows Package Manager repository:

### Method A: Automated via `wingetcreate`
```powershell
wingetcreate new https://github.com/alya-lang/alya/releases/download/v0.0.19/alya-v0.0.19-x86_64-windows.zip
```

### Method B: Manual PR
Copy `alya.yaml` to the community repository under:
`manifests/a/Alya/alya/0.0.19/Alya.alya.yaml`
and submit a Pull Request to [microsoft/winget-pkgs](https://github.com/microsoft/winget-pkgs).
