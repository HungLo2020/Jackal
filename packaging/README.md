# Linux releases and Basalt

Every push to `main` runs `.github/workflows/release.yml`. Pull requests run the
same checks and produce a test artifact but cannot publish. After successful
checks, the publisher creates or updates the single **latest** GitHub release,
moves its `latest` tag to the built commit, and replaces these assets:

- `Jackal-latest-linux-x64.zip`
- `Jackal-latest-linux-x64.zip.sha256`

No version tag or manual release is needed. Commits only exist on GitHub after
they are pushed; local commits cannot trigger GitHub Actions. Superseded builds
are skipped, publishers run serially, and a publisher is not cancelled midway
through an upload. A failed build or test leaves the previous release intact.
If publication itself fails, rerun the failed publish job; uploads are idempotent.
The repository must allow Actions to write releases/tags, and release immutability
must be disabled for this deliberately mutable rolling release. No PAT is needed.

## Installation

Extract the ZIP anywhere and run `app/bin/jackal`. Requires Linux x86-64, glibc
2.35 or newer (Ubuntu 22.04 or equivalent), the system ALSA runtime
(`libasound.so.2`), and an X11 or Wayland desktop. No exact ALSA package version
is required; the `.so.2` suffix identifies its compatible ABI, not a package
version. ALSA and its PipeWire/PulseAudio plugins must come from the same host
distribution so their symbols stay compatible. They are never bundled.

Most desktop installations already provide ALSA. If it is missing, install
`libasound2` or `libasound2t64` on Debian/Ubuntu, or `alsa-lib` on Fedora, using
the distribution's supported package version. Sound uses the desktop's ALSA
configuration/audio service and corresponding host plugins. The launcher reports
a missing ALSA library before starting; it is required even with `--mute`.
No ROM, Rust, Python, emulator or network connection is needed to play.

Runtime requirements specify minimum versions or compatible system ABIs, never
an exact installed package version. CI toolchain versions and Cargo.lock record
reproducible builds; they do not require players to install those build tools.

```
app/
  bin/jackal                 launcher, executable
  bin/jackal-native          Rust executable
  assets/                    native PNG/JSON/WAV assets
  lib/                       bundled desktop client libraries, excluding ALSA
  basalt/cover.jpg           900x1200, opaque, under 2 MB
  basalt/icon.png            512x512, transparent
  basalt/sources.json        artwork provenance and reproduction instructions
  basalt/catalog.json        Basalt schema 2 / standard 1 catalog entry
  licenses/                  bundled library and Rust dependency notices
  release.json               commit, platform, glibc baseline and file hashes
```

The archive contains only `app/`. It never contains `data/`, `.basalt/`, the
source checkout, reference fixtures or a ROM. Unix execute permissions are
preserved in the ZIP. Basalt replaces `app/` on update and preserves user data.

Asset discovery honors `BASALT_APP_DIR`, then `BASALT_GAME_DIR`, then the installed
executable's location. It does not depend on the current directory. Cargo builds
still use source assets. An explicit `--assets` argument overrides discovery.
The launcher resolves symlinks and supplies its bundled library directory.

Jackal currently has no persistent saves, settings, logs or cache. It works with
no `data/` directory and does not create unused placeholder files. Save migration,
atomic save writes and configuration defaults therefore do not apply yet. Future
persistence must use Basalt's category environment variables, falling back to
`data/save`, `data/config`, `data/config/shared` and `data/cache` under the install
root; synced data must be portable and saves must be written atomically. The
explicit `--screenshot` development option writes only to the requested path.
Controller support is intentionally deferred; keyboard controls are in README.md.

## Building the same package locally

CI builds on Ubuntu 22.04 with Rust 1.99.1 and Cargo.lock. A container reproduces
that baseline on a newer Linux host:

```sh
docker build -t jackal-release-builder -f packaging/Dockerfile packaging
docker run --rm --user "$(id -u):$(id -g)" -v "$PWD:/work" \
  -e CARGO_HOME=/tmp/jackal-cargo -e CARGO_TARGET_DIR=/work/target/linux-release \
  jackal-release-builder sh -c 'cargo build --locked --release && python3 tools/release/package.py --binary target/linux-release/release/jackal-native --max-glibc 2.35 && xvfb-run -a python3 tools/release/verify.py dist/Jackal-latest-linux-x64.zip --smoke --gui'
```

On Ubuntu 22.04 directly, install the dependencies listed in the workflow, then:

```sh
cargo build --locked --release
python3 tools/release/package.py --max-glibc 2.35
python3 -m unittest discover -s tools/release -p 'test_*.py'
xvfb-run -a python3 tools/release/verify.py dist/Jackal-latest-linux-x64.zip --smoke --gui
```

The verifier checks archive layout, manifest hashes, permissions, artwork,
catalog matching, and rejects bundled ALSA libraries or plugins. It launches
both the wrapper and ELF from unrelated paths,
tests all six stages with Basalt variables, preserves a user-data sentinel,
checks that `app/` remains unchanged, and opens the real no-argument window then
quits with Escape. Run the smoke checks as an ordinary user for OS enforcement
of the read-only application directory. No release is published by these commands.

## Catalog and artwork

`packaging/basalt.json` matches `resources/games/jackal.json` in Basalt. It requires
no custom actions or layout overrides. Basalt's standard is documented at
https://github.com/HungLo2020/Basalt/blob/main/STANDARDS.md . The pre-install cover
URL references the same image shipped in the archive and becomes available once
these changes are pushed. No Basalt commit or push is part of this setup.

The original NES front-cover scan is hosted by the LaunchBox Games Database:
https://gamesdb.launchbox-app.com/games/images/336-jackal . The source URL and hash
are recorded in `packaging/artwork/sources.json`. The original artwork belongs
to Konami. The icon uses the game's native jeep sprite. To reproduce both images,
download that source JPEG and run `python3 tools/release/prepare_artwork.py FILE`
with Pillow installed. CI uses the committed images and does not download art.

## Basalt standard 1 checklist

- [x] Only `app/` is shipped; user data and Basalt state are absent from the ZIP.
- [x] Assets resolve from Basalt variables or the executable, including moved paths with spaces.
- [x] Normal play writes nothing to `app/` or outside the installation.
- [x] Missing `data/` is supported; existing user files remain untouched.
- [x] No updater, terminal prompts, network dependency, ROM or external language runtime.
- [x] Linux ZIP names are predictable; executable modes and file hashes are validated.
- [x] Desktop client libraries and notices are bundled; ALSA uses the host runtime/plugins.
- [x] Cover and icon are packaged at the specified dimensions; the catalog names the same cover.
- [x] Schema 2 / standard 1 entry validated locally in Basalt, with no layout/action overrides.
- [x] No-argument GUI launch and normal Escape exit tested from an extracted package.
- [ ] A first release is published and installed through Basalt from GitHub: pending the first push.

Save atomicity, migration and configuration-default copying are not applicable
until the game implements persistence. Controller input remains out of scope for
this release. The workflow implements publishing but local validation does not
claim that GitHub Actions or a public download have already run.
