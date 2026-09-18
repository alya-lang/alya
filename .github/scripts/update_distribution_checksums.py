#!/usr/bin/env python3
"""
update_distribution_checksums.py

Automatically updates version numbers and release SHA-256 checksums across
all distribution manifests (Chocolatey, Scoop, and WinGet).
"""

import sys
import re
from pathlib import Path


def update_file(path: Path, pattern: str, replacement: str):
    if not path.is_file():
        print(f"[warn] File not found: {path}")
        return
    content = path.read_text(encoding="utf-8")
    new_content = re.sub(pattern, replacement, content)
    if new_content != content:
        path.write_text(new_content, encoding="utf-8")
        print(f"[updated] {path.name}")
    else:
        print(f"[unchanged] {path.name}")


def main():
    if len(sys.argv) < 3:
        print("Usage: python3 update_distribution_checksums.py <TAG> <SHA256> [REPO_ROOT]")
        sys.exit(1)

    tag = sys.argv[1].strip()
    sha256 = sys.argv[2].strip().lower()
    version = tag.lstrip("v")

    repo_root = (
        Path(sys.argv[3]).resolve()
        if len(sys.argv) > 3
        else Path(__file__).resolve().parent.parent.parent
    )
    dist_dir = repo_root / "packages" / "distribution"

    print(f"Updating distribution manifests for {tag} ({version}) with sha256: {sha256}")

    # 1. Chocolatey nuspec
    nuspec = dist_dir / "chocolatey" / "alya.nuspec"
    update_file(nuspec, r"<version>.*?</version>", f"<version>{version}</version>")
    update_file(
        nuspec,
        r"<releaseNotes>.*?</releaseNotes>",
        f"<releaseNotes>https://github.com/alya-lang/alya/releases/tag/{tag}</releaseNotes>",
    )

    # 2. Chocolatey VERIFICATION.txt
    verif = dist_dir / "chocolatey" / "tools" / "VERIFICATION.txt"
    update_file(
        verif,
        r"releases/download/v[^/]+/alya-v[^/]+-x86_64-windows\.zip",
        f"releases/download/{tag}/alya-{tag}-x86_64-windows.zip",
    )
    update_file(
        verif,
        r"Checksum \(SHA-256\):\s*\n\s*\S+",
        f"Checksum (SHA-256):\n   {sha256}",
    )
    update_file(verif, r"releases/tag/v\S+", f"releases/tag/{tag}")

    # 3. Chocolatey chocolateyinstall.ps1
    choco_ps1 = dist_dir / "chocolatey" / "tools" / "chocolateyinstall.ps1"
    update_file(choco_ps1, r"\$version\s*=\s*'[^']+'", f"$version     = '{version}'")
    update_file(choco_ps1, r"\$checksum64\s*=\s*'[^']+'", f"$checksum64   = '{sha256}'")

    # 4. Chocolatey README.md
    choco_readme = dist_dir / "chocolatey" / "README.md"
    update_file(choco_readme, r"choco push alya\.[^.]+\.[^.]+\.[^.]+\.nupkg", f"choco push alya.{version}.nupkg")

    # 5. Scoop alya.json (parsed via json to preserve autoupdate block)
    scoop_file = dist_dir / "scoop" / "alya.json"
    if scoop_file.is_file():
        import json
        scoop_data = json.loads(scoop_file.read_text(encoding="utf-8"))
        scoop_data["version"] = version
        if "architecture" in scoop_data and "64bit" in scoop_data["architecture"]:
            scoop_data["architecture"]["64bit"]["url"] = f"https://github.com/alya-lang/alya/releases/download/{tag}/alya-{tag}-x86_64-windows.zip"
            scoop_data["architecture"]["64bit"]["hash"] = sha256
            scoop_data["architecture"]["64bit"]["extract_dir"] = f"alya-{tag}-x86_64-windows"
        new_json = json.dumps(scoop_data, indent=2) + "\n"
        if new_json != scoop_file.read_text(encoding="utf-8"):
            scoop_file.write_text(new_json, encoding="utf-8")
            print("[updated] alya.json")
        else:
            print("[unchanged] alya.json")

    # 6. WinGet alya.yaml
    winget = dist_dir / "winget" / "alya.yaml"
    update_file(winget, r"PackageVersion:\s*\S+", f"PackageVersion: {version}")
    update_file(
        winget,
        r"InstallerUrl:\s*https://github\.com/alya-lang/alya/releases/download/[^/]+/alya-[^/]+-x86_64-windows\.zip",
        f"InstallerUrl: https://github.com/alya-lang/alya/releases/download/{tag}/alya-{tag}-x86_64-windows.zip",
    )
    update_file(winget, r"InstallerSha256:\s*\S+", f"InstallerSha256: {sha256}")
    update_file(
        winget,
        r"RelativeFilePath:\s*alya-[^/]+/alya\.exe",
        f"RelativeFilePath: alya-{tag}-x86_64-windows/alya.exe",
    )

    # 7. WinGet README.md
    winget_readme = dist_dir / "winget" / "README.md"
    update_file(
        winget_readme,
        r"releases/download/[^/]+/alya-[^/]+-x86_64-windows\.zip",
        f"releases/download/{tag}/alya-{tag}-x86_64-windows.zip",
    )
    update_file(
        winget_readme,
        r"manifests/a/Alya/alya/[^/]+/Alya\.alya\.yaml",
        f"manifests/a/Alya/alya/{version}/Alya.alya.yaml",
    )


if __name__ == "__main__":
    main()
