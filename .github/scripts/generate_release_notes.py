#!/usr/bin/env python3
"""
generate_release_notes.py
Standardized release notes generator for the Alya compiler repository (alya-lang/alya).

Responsibilities:
1. Resolve release tag and previous tag in git history.
2. Generate commit changelog between releases (What's Changed).
3. Construct compare/commits links for full changelog tracking.
4. Read SHA-256 checksums from dist/ artifacts.
5. Populate .github/release_template.md placeholders and output RELEASE_NOTES.md.
6. Export outputs (tag, title, prev_tag, etc.) to $GITHUB_OUTPUT.
"""

import os
import sys
import subprocess
from pathlib import Path

# Ensure UTF-8 output on all platforms (especially Windows CP1254/CP1252)
if hasattr(sys.stdout, "reconfigure"):
    sys.stdout.reconfigure(encoding="utf-8", errors="replace")
if hasattr(sys.stderr, "reconfigure"):
    sys.stderr.reconfigure(encoding="utf-8", errors="replace")


def run_git(args, check=True):
    """Run git command and return stripped stdout."""
    try:
        res = subprocess.run(
            ["git"] + args,
            capture_output=True,
            text=True,
            check=check,
            encoding="utf-8",
            errors="replace",
        )
        return res.stdout.strip()
    except Exception:
        return ""


def get_checksum(dist_dir, pkg_name):
    """Extract SHA-256 hash from .sha256 file if present."""
    sha_file = dist_dir / f"{pkg_name}.sha256"
    if sha_file.is_file():
        try:
            content = sha_file.read_text(encoding="utf-8").strip()
            if content:
                return content.split()[0]
        except Exception:
            pass
    return "—"


def format_author(name, email):
    """Format author name or GitHub username handle."""
    name = (name or "").strip()
    email = (email or "").strip()
    if "@users.noreply.github.com" in email:
        handle = email.split("@")[0]
        if "+" in handle:
            handle = handle.split("+", 1)[1]
        if handle:
            return f"@{handle}"
    if " " not in name and name:
        return f"@{name}" if not name.startswith("@") else name
    return name or "contributor"


def main():
    # 1. Resolve Release Tag
    tag = ""
    if len(sys.argv) > 1 and sys.argv[1].strip():
        tag = sys.argv[1].strip()
    elif os.environ.get("GITHUB_REF_TYPE") == "tag" and os.environ.get("GITHUB_REF_NAME"):
        tag = os.environ.get("GITHUB_REF_NAME").strip()
    elif os.environ.get("INPUT_TAG"):
        tag = os.environ.get("INPUT_TAG").strip()
    else:
        tag = run_git(["describe", "--tags", "--exact-match"], check=False)
        if not tag:
            tag = run_git(["describe", "--tags", "--abbrev=0"], check=False)

    if not tag:
        tag = "v0.0.18"

    # 2. Resolve Repository Slug (e.g. alya-lang/alya)
    repo = os.environ.get("GITHUB_REPOSITORY", "")
    if not repo:
        origin_url = run_git(["config", "--get", "remote.origin.url"], check=False)
        if "github.com" in origin_url:
            cleaned = origin_url.split("github.com")[-1].lstrip(":").lstrip("/")
            if cleaned.endswith(".git"):
                cleaned = cleaned[:-4]
            repo = cleaned
    if not repo:
        repo = "alya-lang/alya"

    repo_url = f"https://github.com/{repo}"

    # 3. Release Title: "Alya <version>" (e.g. "Alya v0.0.5")
    title_version = tag if tag.startswith("v") else f"v{tag}"
    title = f"Alya {title_version}"

    # 4. Detect Previous Tag in Git History
    # Check whether the target tag already exists as a git object (e.g. tag push).
    # On manual workflow_dispatch triggers, the tag has not been created in git yet,
    # so we compare against HEAD.
    tag_rev = run_git(["rev-parse", "--verify", f"refs/tags/{tag}"], check=False)
    tag_exists = bool(tag_rev)
    target_ref = tag if tag_exists else "HEAD"

    prev_tag = ""
    # If the tag already exists, look before it (tag^). Otherwise, look at HEAD.
    base_ref = f"{tag}^" if tag_exists else target_ref
    describe_prev = run_git(["describe", "--tags", "--abbrev=0", base_ref], check=False)
    if describe_prev and describe_prev != tag:
        prev_tag = describe_prev
    else:
        all_tags_raw = run_git(["tag", "-l", "v*", "--sort=-v:refname"], check=False)
        if all_tags_raw:
            all_tags = [t.strip() for t in all_tags_raw.splitlines() if t.strip() and t.strip() != tag]
            for candidate in all_tags:
                is_ancestor = subprocess.run(
                    ["git", "merge-base", "--is-ancestor", candidate, target_ref],
                    capture_output=True,
                ).returncode == 0
                if is_ancestor:
                    prev_tag = candidate
                    break

    # 5. Full Changelog Link & Commit Range
    if prev_tag:
        full_changelog_url = f"{repo_url}/compare/{prev_tag}...{tag}"
        full_changelog = f"**Full Changelog**: https://github.com/{repo}/compare/{prev_tag}...{tag}"
        commit_range = f"{prev_tag}..{target_ref}"
    else:
        full_changelog_url = f"{repo_url}/commits/{tag}"
        full_changelog = f"**Full Changelog**: https://github.com/{repo}/commits/{tag}"
        commit_range = target_ref

    # 6. Extract Commit Log for Changes Section
    commits_raw = run_git(["log", "--pretty=format:%h%x09%an%x09%ae%x09%s", commit_range], check=False)
    if commits_raw:
        lines = []
        for raw_line in commits_raw.splitlines():
            if not raw_line.strip():
                continue
            parts = raw_line.split("\t", 3)
            if len(parts) == 4:
                commit_hash, author_name, author_email, subject = parts
            elif len(parts) == 3:
                commit_hash, author_name, subject = parts
                author_email = ""
            else:
                continue

            if (
                subject.startswith(f"release {tag}")
                or subject.startswith(f"chore: release {tag}")
                or subject.startswith(f"chore(release): {tag}")
                or "update benchmark results [skip ci]" in subject
            ):
                continue

            author_tag = format_author(author_name, author_email)
            lines.append(f"* {subject} by {author_tag} ({commit_hash})")

        commits_text = "\n".join(lines) if lines else "* Initial release"
    else:
        commits_text = "* Initial release"

    # 7. Checksums from dist/
    dist_dir = Path("dist")
    linux_pkg = f"alyac-{tag}-x86_64-linux.tar.gz"
    mac_arm_pkg = f"alyac-{tag}-arm64-macos.tar.gz"
    mac_x64_pkg = f"alyac-{tag}-x86_64-macos.tar.gz"
    win_pkg = f"alyac-{tag}-x86_64-windows.zip"

    linux_sha = get_checksum(dist_dir, linux_pkg)
    mac_arm_sha = get_checksum(dist_dir, mac_arm_pkg)
    mac_x64_sha = get_checksum(dist_dir, mac_x64_pkg)
    win_sha = get_checksum(dist_dir, win_pkg)

    linux_sha_short = linux_sha[:8] if linux_sha != "—" else "—"
    mac_arm_sha_short = mac_arm_sha[:8] if mac_arm_sha != "—" else "—"
    mac_x64_sha_short = mac_x64_sha[:8] if mac_x64_sha != "—" else "—"
    win_sha_short = win_sha[:8] if win_sha != "—" else "—"

    # 8. Load Template
    template_path = Path(".github/release_template.md")
    if template_path.is_file():
        template = template_path.read_text(encoding="utf-8")
    else:
        template = (
            "Alya is an expressive, compiled, multi-paradigm systems programming language designed for clarity, performance, and simplicity.\n\n"
            "## 🚀 What's Changed\n\n"
            "{{CHANGELOG_COMMITS}}\n\n"
            "## 📦 Pre-built Binaries\n\n"
            "| Platform | Architecture | Package | Checksum |\n"
            "|:---|:---|:---|:---:|\n"
            f"| Linux | `x86_64` | [{linux_pkg}]({repo_url}/releases/download/{{VERSION}}/{linux_pkg}) | [{{{{LINUX_SHA_SHORT}}}}]({repo_url}/releases/download/{{VERSION}}/{linux_pkg}.sha256) |\n"
            f"| macOS | `arm64` (Apple Silicon) | [{mac_arm_pkg}]({repo_url}/releases/download/{{VERSION}}/{mac_arm_pkg}) | [{{{{MAC_ARM_SHA_SHORT}}}}]({repo_url}/releases/download/{{VERSION}}/{mac_arm_pkg}.sha256) |\n"
            f"| macOS | `x86_64` (Intel) | [{mac_x64_pkg}]({repo_url}/releases/download/{{VERSION}}/{mac_x64_pkg}) | [{{{{MAC_X64_SHA_SHORT}}}}]({repo_url}/releases/download/{{VERSION}}/{mac_x64_pkg}.sha256) |\n"
            f"| Windows | `x86_64` | [{win_pkg}]({repo_url}/releases/download/{{VERSION}}/{win_pkg}) | [{{{{WIN_SHA_SHORT}}}}]({repo_url}/releases/download/{{VERSION}}/{win_pkg}.sha256) |\n\n"
            "### 🔒 SHA-256 Checksums\n\n"
            "```text\n"
            f"{{{{LINUX_SHA}}}}  {linux_pkg}\n"
            f"{{{{MAC_ARM_SHA}}}}  {mac_arm_pkg}\n"
            f"{{{{MAC_X64_SHA}}}}  {mac_x64_pkg}\n"
            f"{{{{WIN_SHA}}}}  {win_pkg}\n"
            "```\n\n"
            "---\n\n"
            "{{FULL_CHANGELOG}}\n"
        )

    # 9. Substitute Placeholders
    replacements = {
        "{{VERSION}}": tag,
        "{{RAW_VERSION}}": tag.lstrip("v"),
        "{{REPO}}": repo,
        "{{REPO_URL}}": repo_url,
        "{{LINUX_SHA}}": linux_sha,
        "{{MAC_ARM_SHA}}": mac_arm_sha,
        "{{MAC_X64_SHA}}": mac_x64_sha,
        "{{WIN_SHA}}": win_sha,
        "{{LINUX_SHA_SHORT}}": linux_sha_short,
        "{{MAC_ARM_SHA_SHORT}}": mac_arm_sha_short,
        "{{MAC_X64_SHA_SHORT}}": mac_x64_sha_short,
        "{{WIN_SHA_SHORT}}": win_sha_short,
        "{{PREV_TAG}}": prev_tag,
        "{{CHANGELOG_COMMITS}}": commits_text,
        "{{FULL_CHANGELOG}}": full_changelog,
        "{{CHANGELOG_URL}}": full_changelog_url,
    }

    body = template
    for placeholder, val in replacements.items():
        body = body.replace(placeholder, val)

    # 10. Output to File
    out_file = Path("RELEASE_NOTES.md")
    out_file.write_text(body, encoding="utf-8")
    print(f"Generated release notes in {out_file.resolve()}")
    print(f"  Title:     {title}")
    print(f"  Tag:       {tag}")
    print(f"  Prev Tag:  {prev_tag or '(None - Initial Release)'}")
    print(f"  Changelog: {full_changelog_url}")

    # 11. Export to GITHUB_OUTPUT if present
    github_output = os.environ.get("GITHUB_OUTPUT")
    if github_output:
        with open(github_output, "a", encoding="utf-8") as f:
            f.write(f"tag={tag}\n")
            f.write(f"title={title}\n")
            f.write(f"prev_tag={prev_tag}\n")
            f.write(f"changelog_url={full_changelog_url}\n")


if __name__ == "__main__":
    main()
