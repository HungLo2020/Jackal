"""Packaging boundaries and rolling-release failure behavior, without GitHub writes."""
import hashlib
import json
import os
from pathlib import Path
import shutil
import stat
import subprocess
import tempfile
import unittest
from unittest.mock import patch
import zipfile

import publish
import verify
import package

ROOT = Path(__file__).resolve().parents[2]


class ArchiveTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.archive = Path(self.temporary.name) / "Jackal-latest-linux-x64.zip"

    def make_archive(self, extra=None, executable_mode=0o755, corrupt_manifest=False):
        files = {
            "app/bin/jackal": b"#!/bin/sh\nexit 0\n",
            "app/bin/jackal-native": b"test fixture",
            "app/basalt/cover.jpg": (ROOT / "packaging/artwork/cover.jpg").read_bytes(),
            "app/basalt/icon.png": (ROOT / "packaging/artwork/icon.png").read_bytes(),
            "app/basalt/catalog.json": (ROOT / "packaging/basalt.json").read_bytes(),
        }
        if extra:
            files[extra[0]] = b"unexpected"
        manifest = {"commit": "test", "files": {name: hashlib.sha256(data).hexdigest()
                                                for name, data in files.items()}}
        if corrupt_manifest:
            manifest["files"]["app/bin/jackal-native"] = "0" * 64
        files["app/release.json"] = json.dumps(manifest).encode()
        with zipfile.ZipFile(self.archive, "w") as zipped:
            for name, data in files.items():
                info = zipfile.ZipInfo(name)
                info.create_system = 3
                mode = executable_mode if name.startswith("app/bin/") else 0o644
                info.external_attr = (stat.S_IFREG | mode) << 16
                if extra and name == extra[0] and extra[1]:
                    info.external_attr = (stat.S_IFLNK | 0o777) << 16
                zipped.writestr(info, data)
        self.archive.with_suffix(".zip.sha256").write_text(
            f"{hashlib.sha256(self.archive.read_bytes()).hexdigest()}  {self.archive.name}\n")

    def test_valid_layout_artwork_and_catalog(self):
        self.make_archive()
        self.assertEqual(verify.validate(self.archive)["commit"], "test")

    def test_rejects_user_data_traversal_and_symlinks(self):
        for entry in [("data/save/progress", False), ("app/../data/save", False),
                      ("/app/absolute", False), ("app/.basalt/state", False),
                      ("app/assets/link", True), ("wrapper/app/file", False)]:
            with self.subTest(entry=entry):
                self.make_archive(extra=entry)
                with self.assertRaises(ValueError):
                    verify.validate(self.archive)

    def test_rejects_non_executable_launcher(self):
        self.make_archive(executable_mode=0o644)
        with self.assertRaisesRegex(ValueError, "execute bits"):
            verify.validate(self.archive)

    def test_rejects_content_changed_after_manifest(self):
        self.make_archive(corrupt_manifest=True)
        with self.assertRaisesRegex(ValueError, "manifest digest"):
            verify.validate(self.archive)

    def test_rejects_wrong_external_checksum(self):
        self.make_archive()
        self.archive.with_suffix(".zip.sha256").write_text("wrong checksum")
        with self.assertRaisesRegex(ValueError, "checksum"):
            verify.validate(self.archive)

    def test_rejects_bundled_alsa_and_audio_plugins(self):
        for name in ("libasound.so.2", "libasound.so.2.0.0",
                     "libasound_module_pcm_pipewire.so"):
            with self.subTest(name=name):
                self.make_archive(extra=(f"app/lib/{name}", False))
                with self.assertRaisesRegex(ValueError, "must come from the host"):
                    verify.validate(self.archive)


class HostAudioTests(unittest.TestCase):
    @unittest.skipUnless(shutil.which("cc"), "requires a C compiler for the ELF fixture")
    def test_launcher_reports_missing_host_alsa(self):
        with tempfile.TemporaryDirectory(prefix="jackal missing ALSA ") as temporary:
            app = Path(temporary)
            (app / "bin").mkdir()
            (app / "lib").mkdir()
            binary = app / "bin/jackal-native"
            # Disable default library lookup only for this fixture to simulate
            # a machine without ALSA, without changing the host installation.
            subprocess.run(["cc", "-x", "c", "-", "-Wl,-z,nodefaultlib",
                            "-Wl,--no-as-needed", "-l:libasound.so.2", "-o", str(binary)],
                           input="int main(void) { return 0; }\n", text=True,
                           capture_output=True, check=True)
            for library in package.dependencies(Path("/bin/sh")):
                if library.name == "libc.so.6":
                    shutil.copyfile(library, app / "lib" / library.name)
            launcher = app / "bin/jackal"
            launcher.write_bytes((ROOT / "packaging/jackal").read_bytes())
            launcher.chmod(0o755)
            env = {k: v for k, v in os.environ.items()
                   if k not in {"LD_LIBRARY_PATH", "LD_PRELOAD", "LD_TRACE_LOADED_OBJECTS"}}
            missing = subprocess.run([str(launcher), "--mute"], env=env,
                                     capture_output=True, text=True)
            self.assertEqual(missing.returncode, 1, missing.stderr)
            self.assertIn("requires the system ALSA runtime", missing.stderr)
            self.assertEqual(missing.stdout, "")

    def test_transitive_alsa_dependency_is_not_bundled(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            app = root / "app"
            (app / "lib").mkdir(parents=True)
            binary = root / "jackal-native"
            desktop = root / "libdesktop.so.1"
            desktop.write_bytes(b"desktop library")
            alsa = root / "libasound.so.2"
            plugin = root / "libasound_module_pcm_pipewire.so"
            dependencies = {binary: [desktop], desktop: [alsa, plugin]}
            with patch.object(package, "DESKTOP_LIBRARIES", ()), \
                    patch.object(package, "library_locations", return_value={}), \
                    patch.object(package, "dependencies", side_effect=lambda p: dependencies[p]), \
                    patch.object(package, "copy_library_license"):
                copied = package.bundle_libraries(binary, app)
            self.assertEqual(copied, [desktop.name])
            self.assertEqual([p.name for p in (app / "lib").iterdir()], [desktop.name])

    def test_launcher_forwards_arguments_after_loader_check(self):
        with tempfile.TemporaryDirectory(prefix="jackal launcher ") as temporary:
            app = Path(temporary)
            (app / "bin").mkdir()
            binary = app / "bin/jackal-native"
            # Use an actual ELF so LD_TRACE_LOADED_OBJECTS exercises the loader.
            shutil.copyfile("/bin/sh", binary)
            binary.chmod(0o755)
            launcher = app / "bin/jackal"
            launcher.write_bytes((ROOT / "packaging/jackal").read_bytes())
            launcher.chmod(0o755)
            env = {k: v for k, v in os.environ.items() if k != "LD_TRACE_LOADED_OBJECTS"}
            present = subprocess.run([str(launcher), "-c", 'printf "%s\\n" "$@"',
                                      "test", "--assets", "a path with spaces"],
                                     env=env,
                                     capture_output=True, text=True)
            self.assertEqual(present.returncode, 0, present.stderr)
            self.assertEqual(present.stdout, "--assets\na path with spaces\n")


class PublisherTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.directory = Path(self.temporary.name)
        archive = self.directory / "Jackal-latest-linux-x64.zip"
        with zipfile.ZipFile(archive, "w") as zipped:
            zipped.writestr("app/release.json", json.dumps({"commit": "built"}))
        archive.with_suffix(".zip.sha256").write_text(
            f"{hashlib.sha256(archive.read_bytes()).hexdigest()}  {archive.name}\n")
        self.calls = []
        self.existing = True
        self.newer = False
        self.fail_upload = False

    def gh(self, *args):
        self.calls.append(args)
        if args[:3] == ("release", "upload", "latest") and self.fail_upload:
            raise subprocess.CalledProcessError(1, args)
        if args[0] != "api":
            return ""
        path = args[1]
        if path.endswith("/commits/main"):
            return json.dumps({"sha": "newer" if self.newer else "built"})
        if path.endswith("/releases"):
            return json.dumps([[{"tag_name": "latest"}] if self.existing else []])
        if "matching-refs" in path:
            return json.dumps([{"ref": "refs/tags/latest"}] if self.existing else [])
        if path.endswith("/releases/tags/latest"):
            return json.dumps({"assets": [
                {"id": 1, "name": "Jackal-latest-linux-x64.zip"},
                {"id": 2, "name": "Jackal-old-linux-x64.zip"},
                {"id": 3, "name": "unrelated-notes.txt"},
            ]})
        return "{}"

    def run_publish(self):
        with patch.dict(os.environ, {"GH_REPO": "HungLo2020/Jackal"}), patch.object(publish, "gh", self.gh):
            publish.publish(self.directory, "built")

    def test_existing_release_replaced_without_deleting_release(self):
        self.run_publish()
        upload = next(i for i, c in enumerate(self.calls) if c[:2] == ("release", "upload"))
        tag = next(i for i, c in enumerate(self.calls) if "PATCH" in c)
        edit = next(i for i, c in enumerate(self.calls) if c[:2] == ("release", "edit"))
        self.assertLess(upload, tag)
        self.assertLess(tag, edit)
        self.assertFalse(any(c[:2] == ("release", "delete") for c in self.calls))
        deletions = [c for c in self.calls if "DELETE" in c]
        self.assertEqual(len(deletions), 1)
        self.assertTrue(deletions[0][1].endswith("/assets/2"))

    def test_first_release_stays_draft_until_uploaded(self):
        self.existing = False
        self.run_publish()
        creation = next(c for c in self.calls if c[:2] == ("release", "create"))
        self.assertIn("--draft", creation)
        self.assertTrue(any("POST" in c and "ref=refs/tags/latest" in c for c in self.calls))

    def test_upload_failure_does_not_move_tag_or_publish_draft(self):
        self.fail_upload = True
        with self.assertRaises(subprocess.CalledProcessError):
            self.run_publish()
        self.assertFalse(any("PATCH" in c or c[:2] == ("release", "edit") for c in self.calls))

    def test_superseded_commit_never_mutates_release(self):
        self.newer = True
        self.run_publish()
        self.assertEqual(len(self.calls), 1)

    def test_corrupt_download_never_mutates_release(self):
        (self.directory / "Jackal-latest-linux-x64.zip").write_bytes(b"damaged")
        with self.assertRaisesRegex(ValueError, "checksum"):
            self.run_publish()
        self.assertEqual(len(self.calls), 1)

    def test_valid_checksum_for_wrong_build_never_mutates_release(self):
        archive = self.directory / "Jackal-latest-linux-x64.zip"
        with zipfile.ZipFile(archive, "w") as zipped:
            zipped.writestr("app/release.json", json.dumps({"commit": "wrong"}))
        archive.with_suffix(".zip.sha256").write_text(
            f"{hashlib.sha256(archive.read_bytes()).hexdigest()}  {archive.name}\n")
        with self.assertRaisesRegex(ValueError, "different commit"):
            self.run_publish()
        self.assertEqual(len(self.calls), 1)


if __name__ == "__main__":
    unittest.main()
