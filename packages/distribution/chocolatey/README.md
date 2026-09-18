# Chocolatey Package for Alya Compiler (`alya`)

Official Chocolatey package definition for installing the Alya programming language compiler.

## Building and Testing Locally

### 1. Build the `.nupkg` package
```powershell
# Inside packages/distribution/chocolatey
choco pack
```

### 2. Test installation locally
```powershell
choco install alya --source . -y --force
```

### 3. Verify installation
```powershell
alya --version
alya toolchain status
```

### 4. Test uninstallation
```powershell
choco uninstall alya -y
```

## Publishing to Chocolatey Community Repository

Once tested, push the package to Chocolatey:
```powershell
choco push alya.0.0.18.nupkg --api-key <YOUR_CHOCO_API_KEY> --source https://push.chocolatey.org/
```
