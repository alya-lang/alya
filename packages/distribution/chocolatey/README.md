# Chocolatey Package for Alya Compiler (`alyac`)

Official Chocolatey package definition for installing the Alya programming language compiler.

## Building and Testing Locally

### 1. Build the `.nupkg` package
```powershell
# Inside packages/distribution/chocolatey
choco pack
```

### 2. Test installation locally
```powershell
choco install alyac --source . -y --force
```

### 3. Verify installation
```powershell
alyac --version
alyac toolchain status
```

### 4. Test uninstallation
```powershell
choco uninstall alyac -y
```

## Publishing to Chocolatey Community Repository

Once tested, push the package to Chocolatey:
```powershell
choco push alyac.0.0.18.nupkg --api-key <YOUR_CHOCO_API_KEY> --source https://push.chocolatey.org/
```
