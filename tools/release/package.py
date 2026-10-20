#!/usr/bin/env python3
"""Build a relocatable Basalt ZIP from a Linux x64 release executable.

Run on the oldest supported distribution (Ubuntu 22.04 in CI). Desktop client
libraries, including dlopen dependencies, are bundled; glibc, ALSA and its audio
plugins, and device drivers belong to the host. No Python or Cargo is required
to play the result.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import stat
import subprocess
import tempfile
import zipfile

ROOT = Path(__file__).resolve().parents[2]
DESKTOP_LIBRARIES = (
    "libX11.so.6", "libXcursor.so.1",
    "libwayland-client.so.0", "libwayland-cursor.so.0", "libxkbcommon.so.0",
)
# ALSA must stay matched to the host's dynamically loaded PipeWire/PulseAudio
# plugins. Exclude it even when another bundled library depends on it.
HOST_LIBRARY = re.compile(r"^(?:ld-linux.*|lib(?:c|m|dl|pthread|rt|resolv|util|nss_.*|asound|asound_module_.*)\.so(?:\..*)?)$")


def run(*args):
    return subprocess.check_output(args, text=True, env={**os.environ, "LC_ALL": "C"})


def dependencies(path):
    output = run("ldd", str(path))
    if "not found" in output:
        raise ValueError(f"Unresolved library for {path}:\n{output}")
    return [Path(match.group(1)) for match in re.finditer(r"=> (/\S+)", output)]


def library_locations():
    output = run("ldconfig", "-p")
    result = {}
    for line in output.splitlines():
        match = re.match(r"\s*(\S+) \(.*x86-64.*\) => (.+)", line)
        if match:
            result.setdefault(match[1], Path(match[2]))
    return result


def copy_library_license(source, destination):
    # Debian/Ubuntu paths differ before/after usrmerge; try both spellings.
    candidates = [str(source), str(source.resolve())]
    candidates += [p.removeprefix("/usr") for p in candidates if p.startswith("/usr/lib/")]
    for candidate in candidates:
        query = subprocess.run(["dpkg-query", "-S", candidate], text=True, capture_output=True)
        if query.returncode == 0:
            package = query.stdout.split(": ", 1)[0].split(":", 1)[0]
            license_file = Path("/usr/share/doc") / package / "copyright"
            if license_file.is_file():
                destination.mkdir(parents=True, exist_ok=True)
                shutil.copyfile(license_file, destination / f"{package}.copyright")
                return
    raise ValueError(f"Cannot locate redistribution notices for {source}")


def bundle_libraries(binary, app):
    locations = library_locations()
    queue = dependencies(binary)
    for name in DESKTOP_LIBRARIES:
        if name not in locations:
            raise ValueError(f"Install the build dependency providing {name}")
        queue.append(locations[name])
    copied = set()
    while queue:
        path = queue.pop()
        if path.name in copied or HOST_LIBRARY.fullmatch(path.name):
            continue
        copied.add(path.name)
        queue.extend(dependencies(path))
        shutil.copyfile(path, app / "lib" / path.name)
        copy_library_license(path, app / "licenses/libraries")
    return sorted(copied)


def glibc_requirement(paths):
    versions = set()
    for path in paths:
        output = run("readelf", "--version-info", str(path))
        versions.update(tuple(map(int, value.split("."))) for value in re.findall(r"GLIBC_(\d+\.\d+)", output))
    return max(versions, default=(0, 0))


def copy_rust_notices(app):
    metadata = json.loads(run("cargo", "metadata", "--locked", "--format-version", "1",
                              "--filter-platform", "x86_64-unknown-linux-gnu"))
    records = []
    for package in metadata["packages"]:
        if package["source"] is None:
            continue
        directory = Path(package["manifest_path"]).parent
        destination = app / "licenses/rust" / f"{package['name']}-{package['version']}"
        notices = [p for p in directory.iterdir() if p.is_file()
                   and p.name.upper().startswith(("LICENSE", "LICENCE", "COPYING", "NOTICE", "UNLICENSE"))]
        if package.get("license_file"):
            notices.append(directory / package["license_file"])
        if not notices:
            # Some crates publish their licensing text in the README itself.
            notices = [p for p in directory.glob("README*") if p.is_file()]
        for notice in notices:
            destination.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(notice, destination / notice.name)
        records.append({"name": package["name"], "version": package["version"],
                        "license": package["license"], "repository": package["repository"]})
    (app / "licenses/rust-dependencies.json").write_text(json.dumps(records, indent=2) + "\n")


def build(binary, output, commit, max_glibc=None):
    if "Advanced Micro Devices X86-64" not in run("readelf", "-h", str(binary)):
        raise ValueError("Expected a Linux x86-64 ELF executable")
    output.mkdir(parents=True, exist_ok=True)
    archive = output / "Jackal-latest-linux-x64.zip"
    with tempfile.TemporaryDirectory(prefix="jackal-package-") as temporary:
        root = Path(temporary)
        app = root / "app"
        (app / "bin").mkdir(parents=True)
        (app / "lib").mkdir()
        shutil.copyfile(binary, app / "bin/jackal-native")
        subprocess.run(["strip", "--strip-unneeded", str(app / "bin/jackal-native")], check=True)
        shutil.copyfile(ROOT / "packaging/jackal", app / "bin/jackal")
        for path in (app / "bin").iterdir():
            path.chmod(0o755)
        shutil.copytree(ROOT / "assets", app / "assets")
        shutil.copytree(ROOT / "packaging/artwork", app / "basalt")
        shutil.copyfile(ROOT / "packaging/basalt.json", app / "basalt/catalog.json")
        shutil.copyfile(ROOT / "README.md", app / "README.md")
        shutil.copyfile(ROOT / "packaging/README.md", app / "RELEASE.md")
        libraries = bundle_libraries(app / "bin/jackal-native", app)
        copy_rust_notices(app)
        glibc = glibc_requirement([app / "bin/jackal-native", *(app / "lib").iterdir()])
        if max_glibc and glibc > tuple(map(int, max_glibc.split("."))):
            raise ValueError(f"Requires glibc {glibc}; build on the documented baseline instead")
        files = {}
        for path in sorted(app.rglob("*")):
            if path.is_symlink():
                raise ValueError(f"Unexpected symlink: {path}")
            if path.is_file():
                if path.suffix.lower() in {".nes", ".bin"}:
                    raise ValueError(f"Unexpected legacy ROM/binary asset: {path}")
                files[path.relative_to(root).as_posix()] = hashlib.sha256(path.read_bytes()).hexdigest()
        manifest = {
            "game": "Jackal", "commit": commit, "platform": "linux-x64",
            "basalt_standard_version": 1, "minimum_glibc": ".".join(map(str, glibc)),
            "libraries": libraries, "files": files,
        }
        (app / "release.json").write_text(json.dumps(manifest, indent=2) + "\n")
        with zipfile.ZipFile(archive, "w", zipfile.ZIP_DEFLATED, compresslevel=6) as zipped:
            for path in sorted(app.rglob("*")):
                if path.is_file():
                    info = zipfile.ZipInfo(path.relative_to(root).as_posix(), (2020, 1, 1, 0, 0, 0))
                    info.create_system = 3
                    mode = 0o755 if path.parent == app / "bin" else 0o644
                    info.external_attr = (stat.S_IFREG | mode) << 16
                    info.compress_type = zipfile.ZIP_DEFLATED
                    zipped.writestr(info, path.read_bytes())
    digest = hashlib.sha256(archive.read_bytes()).hexdigest()
    archive.with_suffix(".zip.sha256").write_text(f"{digest}  {archive.name}\n")
    print(f"Created {archive} ({archive.stat().st_size:,} bytes; glibc {manifest['minimum_glibc']})")
    return archive


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", type=Path, default=ROOT / "target/release/jackal-native")
    parser.add_argument("--output", type=Path, default=ROOT / "dist")
    parser.add_argument("--commit", default="local-unstaged")
    parser.add_argument("--max-glibc", help="Fail if the package exceeds this glibc baseline")
    args = parser.parse_args()
    build(args.binary, args.output, args.commit, args.max_glibc)


if __name__ == "__main__":
    main()
