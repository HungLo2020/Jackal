#!/usr/bin/env python3
"""Replace the rolling GitHub release. Called only by the trusted publish job."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
import tempfile
import zipfile


def gh(*args):
    return subprocess.check_output(["gh", *args], text=True)


def api(path, *args):
    return json.loads(gh("api", path, *args))


def publish(directory, commit):
    repository = os.environ["GH_REPO"]
    base = f"repos/{repository}"
    if api(f"{base}/commits/main")["sha"] != commit:
        print("A newer commit is on main; skipping this superseded release")
        return
    archive = directory / "Jackal-latest-linux-x64.zip"
    checksum = archive.with_suffix(".zip.sha256")
    if checksum.read_text().split() != [hashlib.sha256(archive.read_bytes()).hexdigest(), archive.name]:
        raise ValueError("Downloaded artifact checksum mismatch")
    with zipfile.ZipFile(archive) as zipped:
        if json.loads(zipped.read("app/release.json"))["commit"] != commit:
            raise ValueError("Artifact was built from a different commit")

    # Listing lets authentication/network errors fail rather than masquerade as
    # a missing release. An existing release is edited in place, not deleted.
    releases = json.loads(gh("api", f"{base}/releases", "--paginate", "--slurp"))
    existing = next((release for page in releases for release in page if release["tag_name"] == "latest"), None)
    with tempfile.TemporaryDirectory(prefix="jackal-release-notes-") as temporary:
        notes = Path(temporary) / "notes.md"
        notes.write_text(
            f"Rolling Linux x64 build from `{commit}`.\n\n"
            "Download the ZIP and extract it, then run `app/bin/jackal`, or install through Basalt.\n"
            "Requires an x86-64 Linux desktop with glibc 2.35 or newer and the system ALSA runtime "
            "(libasound.so.2) with its desktop audio plugins. No exact ALSA package version is required. "
            "No ROM, Rust, or emulator is needed.\n\n"
            "The ZIP includes the game, native assets, runtime libraries, cover and icon. "
            "The SHA-256 checksum is attached separately. This release is replaced after successful main-branch pushes.\n"
        )
        if existing is None:
            gh("release", "create", "latest", "--target", commit, "--draft",
               "--title", "Jackal — Latest Linux build", "--notes-file", str(notes))
        # Test/build failures occur before this job. Interrupted uploads fail the
        # job; a rerun safely replaces these same names with --clobber.
        gh("release", "upload", "latest", str(archive), str(checksum), "--clobber")
        # GitHub does not move an existing tag when --target is edited.
        refs = api(f"{base}/git/matching-refs/tags/latest")
        if any(ref["ref"] == "refs/tags/latest" for ref in refs):
            api(f"{base}/git/refs/tags/latest", "--method", "PATCH", "-f", f"sha={commit}", "-F", "force=true")
        else:
            api(f"{base}/git/refs", "--method", "POST", "-f", "ref=refs/tags/latest", "-f", f"sha={commit}")
        gh("release", "edit", "latest", "--target", commit, "--draft=false", "--prerelease=false",
           "--latest", "--title", "Jackal — Latest Linux build", "--notes-file", str(notes))
        # Keep exactly one Basalt-matching archive, even if an older workflow
        # used versioned filenames. Other manually attached assets are retained.
        release = api(f"{base}/releases/tags/latest")
        for asset in release["assets"]:
            name = asset["name"]
            if (name.startswith("Jackal-") and (name.endswith(".zip") or name.endswith(".zip.sha256"))
                    and name not in {archive.name, checksum.name}):
                gh("api", f"{base}/releases/assets/{asset['id']}", "--method", "DELETE")
    print(f"Updated rolling latest release to {commit}")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--directory", type=Path, default=Path("dist"))
    parser.add_argument("--commit", required=True)
    args = parser.parse_args()
    publish(args.directory, args.commit)
