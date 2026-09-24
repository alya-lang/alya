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
    if len(sys.argv) < 4:
        print("Usage: python3 update_distribution_checksums.py <TAG> <SHA256_X64> <SHA256_ARM64> [REPO_ROOT]")
        sys.exit(1)

    tag = sys.argv[1].strip()
    sha256_x64 = sys.argv[2].strip().lower()
    sha256_arm64 = sys.argv[3].strip().lower()
    version = tag.lstrip("v")

    repo_root = (
        Path(sys.argv[4]).resolve()
        if len(sys.argv) > 4
        else Path(__file__).resolve().parent.parent.parent
    )
    dist_dir = repo_root / "packages" / "distribution"

    print(f"Updating distribution manifests for {tag} ({version})")
    print(f"  x64:   {sha256_x64}")
    print(f"  ARM64: {sha256_arm64}")

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
        r"releases/download/v[^/\s]+/",
        f"releases/download/{tag}/",
    )
    update_file(
        verif,
        r"alya-v[^/\s]+?-x86_64-windows\.zip",
        f"alya-{tag}-x86_64-windows.zip",
    )
    update_file(
        verif,
        r"alya-v[^/\s]+?-arm64-windows\.zip",
        f"alya-{tag}-arm64-windows.zip",
    )
    update_file(
        verif,
        r"x64:\s*[0-9a-fA-F]{64}",
        f"x64:   {sha256_x64}",
    )
    update_file(
        verif,
        r"ARM64:\s*[0-9a-fA-F]{64}",
        f"ARM64: {sha256_arm64}",
    )
    update_file(verif, r"releases/tag/v\S+", f"releases/tag/{tag}")

    # 3. Chocolatey chocolateyinstall.ps1
    choco_ps1 = dist_dir / "chocolatey" / "tools" / "chocolateyinstall.ps1"
    update_file(choco_ps1, r"\$version\s*=\s*'[^']+'", f"$version     = '{version}'")
    update_file(choco_ps1, r"\$checksum64\s*=\s*'[^']+'", f"$checksum64   = '{sha256_x64}'")
    update_file(choco_ps1, r"\$checksumArm64\s*=\s*'[^']+'", f"$checksumArm64 = '{sha256_arm64}'")

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
            scoop_data["architecture"]["64bit"]["hash"] = sha256_x64
            scoop_data["architecture"]["64bit"]["extract_dir"] = f"alya-{tag}-x86_64-windows"
        if "architecture" in scoop_data and "arm64" in scoop_data["architecture"]:
            scoop_data["architecture"]["arm64"]["url"] = f"https://github.com/alya-lang/alya/releases/download/{tag}/alya-{tag}-arm64-windows.zip"
            scoop_data["architecture"]["arm64"]["hash"] = sha256_arm64
            scoop_data["architecture"]["arm64"]["extract_dir"] = f"alya-{tag}-arm64-windows"
        new_json = json.dumps(scoop_data, indent=2) + "\n"
        if new_json != scoop_file.read_text(encoding="utf-8"):
            scoop_file.write_text(new_json, encoding="utf-8")
            print("[updated] alya.json")
        else:
            print("[unchanged] alya.json")

    # 6. WinGet alya.yaml (x64 + ARM64 installers)
    winget = dist_dir / "winget" / "alya.yaml"
    update_file(winget, r"PackageVersion:\s*\S+", f"PackageVersion: {version}")
    update_file(
        winget,
        r"InstallerUrl:\s*https://github\.com/alya-lang/alya/releases/download/[^/]+/alya-[^/]+-x86_64-windows\.zip",
        f"InstallerUrl: https://github.com/alya-lang/alya/releases/download/{tag}/alya-{tag}-x86_64-windows.zip",
    )
    update_file(
        winget,
        r"InstallerUrl:\s*https://github\.com/alya-lang/alya/releases/download/[^/]+/alya-[^/]+-arm64-windows\.zip",
        f"InstallerUrl: https://github.com/alya-lang/alya/releases/download/{tag}/alya-{tag}-arm64-windows.zip",
    )
    update_file(
        winget,
        r"RelativeFilePath:\s*alya-[^/]+-x86_64-windows/alya\.exe",
        f"RelativeFilePath: alya-{tag}-x86_64-windows/alya.exe",
    )
    update_file(
        winget,
        r"RelativeFilePath:\s*alya-[^/]+-arm64-windows/alya\.exe",
        f"RelativeFilePath: alya-{tag}-arm64-windows/alya.exe",
    )
    # InstallerSha256 lines are positional: first belongs to x64, second to ARM64.
    if winget.is_file():
        content = winget.read_text(encoding="utf-8")
        matches = list(re.finditer(r"InstallerSha256:\s*[0-9a-fA-F]{64}", content))
        if len(matches) != 2:
            print(f"[warn] Expected 2 InstallerSha256 lines in alya.yaml, found {len(matches)}")
        else:
            new_content = (
                content[: matches[0].start()]
                + f"InstallerSha256: {sha256_x64}"
                + content[matches[0].end() : matches[1].start()]
                + f"InstallerSha256: {sha256_arm64}"
                + content[matches[1].end() :]
            )
            if new_content != content:
                winget.write_text(new_content, encoding="utf-8")
                print("[updated] alya.yaml (sha256)")
            else:
                print("[unchanged] alya.yaml (sha256)")

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
