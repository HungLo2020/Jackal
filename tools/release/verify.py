#!/usr/bin/env python3
"""Validate a release ZIP and run its relocated executable without source assets."""
import argparse
import hashlib
import io
import json
import os
from pathlib import Path, PurePosixPath
import stat
import subprocess
import tempfile
import zipfile

from PIL import Image


def validate(archive):
    with zipfile.ZipFile(archive) as zipped:
        names = zipped.namelist()
        if len(names) != len(set(names)):
            raise ValueError("Duplicate archive entries")
        for info in zipped.infolist():
            path = PurePosixPath(info.filename)
            mode = info.external_attr >> 16
            if path.name.startswith(("libasound.so", "libasound_module_")):
                raise ValueError("ALSA and its plugins must come from the host, not the release")
            if (not path.parts or path.parts[0] != "app" or path.is_absolute()
                    or ".." in path.parts or "\\" in info.filename
                    or ".basalt" in path.parts or "data" in path.parts
                    or not stat.S_ISREG(mode)):
                raise ValueError(f"Invalid release entry: {info.filename}")
        for executable in ("app/bin/jackal", "app/bin/jackal-native"):
            if zipped.getinfo(executable).external_attr >> 16 & 0o111 != 0o111:
                raise ValueError(f"Missing execute bits: {executable}")
        manifest = json.loads(zipped.read("app/release.json"))
        if set(names) != set(manifest["files"]) | {"app/release.json"}:
            raise ValueError("Manifest does not describe exactly the released files")
        for name, digest in manifest["files"].items():
            if hashlib.sha256(zipped.read(name)).hexdigest() != digest:
                raise ValueError(f"Incorrect manifest digest: {name}")
        cover_bytes = zipped.read("app/basalt/cover.jpg")
        cover = Image.open(io.BytesIO(cover_bytes))
        if cover.size != (900, 1200) or cover.mode != "RGB" or len(cover_bytes) >= 2_000_000:
            raise ValueError("Cover must be opaque RGB, 900x1200, below 2 MB")
        icon = Image.open(io.BytesIO(zipped.read("app/basalt/icon.png")))
        if icon.size != (512, 512) or icon.mode != "RGBA" or icon.getextrema()[3] != (0, 255):
            raise ValueError("Icon must be transparent RGBA, 512x512")
        catalog = json.loads(zipped.read("app/basalt/catalog.json"))
        if (catalog["schema_version"], catalog["standard_version"]) != (2, 1):
            raise ValueError("Incorrect Basalt catalog schema")
        source = catalog["source"]
        if not (archive.name.startswith(source["asset_prefix"])
                and archive.name.endswith(source["asset_suffix"])
                and all(part in archive.name for part in source["asset_contains"])):
            raise ValueError("Basalt catalog cannot match this archive")
        if zipped.testzip() is not None:
            raise ValueError("Corrupt archive")
    checksum = archive.with_suffix(".zip.sha256").read_text().split()
    if checksum != [hashlib.sha256(archive.read_bytes()).hexdigest(), archive.name]:
        raise ValueError("Release checksum does not match")
    return manifest


def smoke(archive, gui=False):
    with tempfile.TemporaryDirectory(prefix="jackal release test ") as temporary:
        root = Path(temporary)
        install = root / "moved library" / "Jackal"
        install.mkdir(parents=True)
        with zipfile.ZipFile(archive) as zipped:
            zipped.extractall(install)
            for info in zipped.infolist():
                (install / info.filename).chmod((info.external_attr >> 16) & 0o777)
        app = install / "app"
        for path in app.rglob("*"):
            path.chmod(0o555 if path.is_dir() or path.parent == app / "bin" else 0o444)
        app.chmod(0o555)
        unrelated = root / "unrelated working directory"
        unrelated.mkdir()
        env = {k: v for k, v in os.environ.items() if not k.startswith("BASALT_")}
        env["HOME"] = str(root / "isolated home")
        Path(env["HOME"]).mkdir()
        before = {str(p.relative_to(app)): hashlib.sha256(p.read_bytes()).hexdigest()
                  for p in app.rglob("*") if p.is_file()}
        launcher = app / "bin/jackal"
        command = [str(launcher), "--headless", "1200", "--stage", "1", "--mute"]
        try:
            subprocess.run(command, cwd=unrelated, env=env, check=True, timeout=90)
            # The ELF itself must also find assets, independently of the launcher.
            subprocess.run([str(app / "bin/jackal-native"), "--headless", "1", "--mute"],
                           cwd=unrelated, env={**env, "LD_LIBRARY_PATH": str(app / "lib")},
                           check=True, timeout=90)
            basalt_env = {**env, "BASALT_GAME_ID": "Jackal", "BASALT_STANDARD_VERSION": "1",
                          "BASALT_GAME_DIR": str(install), "BASALT_APP_DIR": str(app),
                          "BASALT_SAVE_DIR": str(install / "data/save"),
                          "BASALT_CONFIG_DIR": str(install / "data/config"),
                          "BASALT_SHARED_CONFIG_DIR": str(install / "data/config/shared"),
                          "BASALT_CACHE_DIR": str(install / "data/cache")}
            for stage in range(1, 7):
                subprocess.run([str(launcher), "--headless", "1200", "--stage", str(stage), "--mute"],
                               cwd=install, env=basalt_env, check=True, timeout=90)
            # Existing user data must survive a subsequent start byte-for-byte.
            sentinel = install / "data/save/keep.txt"
            sentinel.parent.mkdir(parents=True, exist_ok=True)
            sentinel.write_text("user data\n")
            subprocess.run(command, cwd=install, env=basalt_env, check=True, timeout=90)
            assert sentinel.read_text() == "user data\n"
            if gui:
                # Exercise the real no-argument launch, then quit via Escape.
                process = subprocess.Popen([str(launcher)], cwd=unrelated, env=env)
                try:
                    window = subprocess.check_output(
                        ["timeout", "20", "xdotool", "search", "--sync", "--onlyvisible", "--class", "JackalNative"],
                        env=env, text=True).splitlines()[0]
                    import time
                    time.sleep(2)
                    # Xvfb has no window manager to focus the new window. Hold
                    # Escape across multiple frames so polling cannot miss it.
                    subprocess.run(["xdotool", "windowfocus", "--sync", window], env=env, check=True, timeout=5)
                    subprocess.run(["xdotool", "keydown", "Escape"], env=env, check=True, timeout=5)
                    time.sleep(0.2)
                    subprocess.run(["xdotool", "keyup", "Escape"], env=env, check=True, timeout=5)
                    if process.wait(timeout=15) != 0:
                        raise ValueError("Normal window quit failed")
                finally:
                    if process.poll() is None:
                        process.kill()
                        process.wait()
            after = {str(p.relative_to(app)): hashlib.sha256(p.read_bytes()).hexdigest()
                     for p in app.rglob("*") if p.is_file()}
            if before != after:
                raise ValueError("Game changed the read-only application tree")
            if list(Path(env["HOME"]).rglob("*")):
                raise ValueError("Game wrote outside its install root")
        finally:
            for path in app.rglob("*"):
                if path.is_dir():
                    path.chmod(0o755)
            app.chmod(0o755)
    print("Relocated release passed all six stages, read-only app and user-data checks")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("archive", type=Path)
    parser.add_argument("--smoke", action="store_true")
    parser.add_argument("--gui", action="store_true", help="Also test no-argument launch on DISPLAY")
    args = parser.parse_args()
    manifest = validate(args.archive)
    print(f"Validated {len(manifest['files'])} files; commit {manifest['commit']}")
    if args.smoke or args.gui:
        smoke(args.archive, args.gui)


if __name__ == "__main__":
    main()
