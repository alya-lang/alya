# Scoop Manifest for Alya Compiler (`alyac`)

Official Scoop manifest definition for installing the Alya programming language compiler.

## Installing via Scoop

### Option 1: Install directly from manifest URL or file
```powershell
scoop install https://raw.githubusercontent.com/alya-lang/alya/main/packages/distribution/scoop/alyac.json
```

### Option 2: Install from local repository
```powershell
scoop install ./alyac.json
```

## Verify Installation
```powershell
alyac --version
alyac toolchain status
```
