# Alya Compiler Distribution Packages

This directory contains package definitions and manifests for Windows package managers:

| Package Manager | Directory | Target Package | Status |
|:---|:---|:---|:---:|
| **WinGet** | [`winget/`](winget/) | `Alya.alya` | Ready |
| **Chocolatey** | [`chocolatey/`](chocolatey/) | `alya` | Ready |
| **Scoop** | [`scoop/`](scoop/) | `alya` | Ready |

---

## 🪟 Windows Package Manager (WinGet)
Install via local manifest or community repo:
```powershell
# Local validation & test
winget validate --manifest packages/distribution/winget/alya.yaml
winget install --manifest packages/distribution/winget/alya.yaml
```

## 🍫 Chocolatey (`choco`)
Build and test `.nupkg`:
```powershell
cd packages/distribution/chocolatey
choco pack
choco install alya --source . -y
```

## 🍨 Scoop
Install via Scoop bucket or direct URL:
```powershell
scoop install https://raw.githubusercontent.com/alya-lang/alya/main/packages/distribution/scoop/alya.json
```
