Alya is an expressive, compiled, multi-paradigm systems programming language designed for clarity, performance, and simplicity.

## 🚀 What's Changed

{{CHANGELOG_COMMITS}}

## 📦 Pre-built Binaries

| Platform | Architecture | Package | Checksum |
|:---|:---|:---|:---:|
| <img src="https://svgl.app/library/linux.svg" width="16" height="16" valign="middle" alt="Linux" />&nbsp;**Linux** | `x86_64` | [alya-{{VERSION}}-x86_64-linux.tar.gz](https://github.com/{{REPO}}/releases/download/{{VERSION}}/alya-{{VERSION}}-x86_64-linux.tar.gz) | [`{{LINUX_SHA_SHORT}}`](https://github.com/{{REPO}}/releases/download/{{VERSION}}/alya-{{VERSION}}-x86_64-linux.tar.gz.sha256) |
| <picture><source media="(prefers-color-scheme: dark)" srcset="https://svgl.app/library/apple_dark.svg"><source media="(prefers-color-scheme: light)" srcset="https://svgl.app/library/apple.svg"><img src="https://svgl.app/library/apple.svg" width="16" height="16" valign="middle" alt="macOS" /></picture>&nbsp;**macOS** | `arm64` (Apple Silicon) | [alya-{{VERSION}}-arm64-macos.tar.gz](https://github.com/{{REPO}}/releases/download/{{VERSION}}/alya-{{VERSION}}-arm64-macos.tar.gz) | [`{{MAC_ARM_SHA_SHORT}}`](https://github.com/{{REPO}}/releases/download/{{VERSION}}/alya-{{VERSION}}-arm64-macos.tar.gz.sha256) |
| <picture><source media="(prefers-color-scheme: dark)" srcset="https://svgl.app/library/apple_dark.svg"><source media="(prefers-color-scheme: light)" srcset="https://svgl.app/library/apple.svg"><img src="https://svgl.app/library/apple.svg" width="16" height="16" valign="middle" alt="macOS" /></picture>&nbsp;**macOS** | `x86_64` (Intel) | [alya-{{VERSION}}-x86_64-macos.tar.gz](https://github.com/{{REPO}}/releases/download/{{VERSION}}/alya-{{VERSION}}-x86_64-macos.tar.gz) | [`{{MAC_X64_SHA_SHORT}}`](https://github.com/{{REPO}}/releases/download/{{VERSION}}/alya-{{VERSION}}-x86_64-macos.tar.gz.sha256) |
| <img src="https://svgl.app/library/windows.svg" width="16" height="16" valign="middle" alt="Windows" />&nbsp;**Windows** | `x86_64` | [alya-{{VERSION}}-x86_64-windows.zip](https://github.com/{{REPO}}/releases/download/{{VERSION}}/alya-{{VERSION}}-x86_64-windows.zip) | [`{{WIN_SHA_SHORT}}`](https://github.com/{{REPO}}/releases/download/{{VERSION}}/alya-{{VERSION}}-x86_64-windows.zip.sha256) |

### 🔒 SHA-256 Checksums

```text
{{LINUX_SHA}}  alya-{{VERSION}}-x86_64-linux.tar.gz
{{MAC_ARM_SHA}}  alya-{{VERSION}}-arm64-macos.tar.gz
{{MAC_X64_SHA}}  alya-{{VERSION}}-x86_64-macos.tar.gz
{{WIN_SHA}}  alya-{{VERSION}}-x86_64-windows.zip
```

---

## ⚡ Quick Start

### Linux / macOS
```bash
# 1. Extract the archive
tar -xzf alya-{{VERSION}}-<platform>.tar.gz
cd alya-{{VERSION}}-<platform>

# 2. Check compiler version
./alya --version

# 3. Run an Alya program directly
./alya run main.alya

# 4. Or compile to a native binary
./alya build main.alya
```

### Windows (PowerShell)
```powershell
# 1. Extract the archive
Expand-Archive alya-{{VERSION}}-x86_64-windows.zip
cd alya-{{VERSION}}-x86_64-windows

# 2. Check compiler version
.\alya.exe --version

# 3. Run an Alya program directly
.\alya.exe run main.alya
```

---

## 🔒 Checksum Verification

```bash
# Linux / macOS
shasum -a 256 -c alya-{{VERSION}}-<platform>.tar.gz.sha256

# Windows (PowerShell)
(Get-FileHash alya-{{VERSION}}-x86_64-windows.zip -Algorithm SHA256).Hash.ToLower()
```

---

## 🔗 Useful Links

- **Documentation**: [https://github.com/{{REPO}}#readme](https://github.com/{{REPO}}#readme)
- **Standard Library**: [std/](https://github.com/{{REPO}}/tree/{{VERSION}}/std)
- **Examples**: [examples/](https://github.com/{{REPO}}/tree/{{VERSION}}/examples)
- **Issue Tracker**: [GitHub Issues](https://github.com/{{REPO}}/issues)

---

{{FULL_CHANGELOG}}
