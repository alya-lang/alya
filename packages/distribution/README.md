# Alya Compiler Distribution Packages

This directory contains package definitions and manifests for Windows package managers:

| Package Manager | Directory | Target Package | Status |
|:---|:---|:---|:---:|
| **WinGet** | [`winget/`](winget/) | `Alya.alyac` | Ready |
| **Chocolatey** | [`chocolatey/`](chocolatey/) | `alyac` | Ready |
| **Scoop** | [`scoop/`](scoop/) | `alyac` | Ready |

---

## 🪟 Windows Package Manager (WinGet)
Install via local manifest or community repo:
```powershell
# Local validation & test
winget validate --manifest packages/distribution/winget/alyac.yaml
winget install --manifest packages/distribution/winget/alyac.yaml
```

## 🍫 Chocolatey (`choco`)
Build and test `.nupkg`:
```powershell
cd packages/distribution/chocolatey
choco pack
choco install alyac --source . -y
```

## 🍨 Scoop
Install via Scoop bucket or direct URL:
```powershell
scoop install https://raw.githubusercontent.com/alya-lang/alya/main/packages/distribution/scoop/alyac.json
```
