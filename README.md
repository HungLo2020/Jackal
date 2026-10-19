# Jackal — native Rust reconstruction

```sh
cargo run
```

The game uses ordinary Rust gameplay systems and checked-in PNG, JSON, and PCM
WAV assets. No ROM, emulator, generated instruction dispatcher, banked data image,
Python, or asset extraction step is needed to build or play. Linux builds need
the desktop/ALSA development libraries used by minifb and rodio.

Linux releases use Basalt's standard layout. Download
`Jackal-latest-linux-x64.zip` from the repository's **latest** release, extract it,
and run `app/bin/jackal`; the package includes assets, artwork and runtime client
libraries. The rolling release is replaced after successful pushes to `main`.
See [release packaging and Basalt compatibility](packaging/README.md) for the
layout, system requirements, validation and local build commands. The workflow
and pre-install cover URL become active when these files are committed and pushed.

This is a playable six-stage reconstruction with two-player controls, prisoners,
weapon upgrades, destructible terrain, bosses, continues, and an ending loop.
The fidelity correction is in progress: original presentation and recovered actor
state machines are replacing the earlier approximations. The remaining gaps are
tracked in `analysis/fidelity-audit.md`; this is not yet a claim of complete parity.

## Controls

| Action | Player 1 | Player 2 |
|---|---|---|
| Move | Arrow keys | WASD |
| Grenade / upgraded weapon | Z | G |
| Machine gun | X | H |
| Start | Enter | Y |
| Select players | Tab | T |

P pauses, R returns to the title, and Escape exits. Select chooses one or two
players on the title screen. The window is resizable and retains the full-opacity
X11 application hints. `cargo run -- --mute` disables sound; unavailable audio
hardware does not prevent play.

## Source layout

| File | Responsibility |
|---|---|
| `src/main.rs` | CLI, input, fixed-timestep loop, window and sound event delivery |
| `src/game.rs`, `src/game/` | Named scene states, campaign progression, spawns and system coordination |
| `src/actor_pool.rs` | Shared actor capacity, stable slot ownership and priority eviction |
| `src/player.rs`, `src/camera.rs` | Recovered jeep movement, scrolling, co-op boundaries and progression |
| `src/cinematic.rs`, `src/victory_scene.rs`, `src/hud.rs` | Original scene timelines, live rescue tally and sprite HUD |
| `src/rescue.rs`, `src/rescue_helicopter.rs` | Prisoner emergence, wandering, boarding and helicopter states |
| `src/weapon.rs` | Fire cadence, projectile motion, grenade and rocket blasts |
| `src/enemy.rs` | Native enemy families using editable entity definitions |
| `src/boss.rs` | Six encounters, armor, delayed arrivals and core shields |
| `src/terrain.rs` | Destruction patches shared by rendering and collision |
| `src/renderer.rs` | PNG atlas composition, camera, sprites, HUD and menus |
| `src/sound.rs` | WAV playback, music loops, effects and pause |
| `src/assets.rs` | Typed JSON loading and validation |

`Game::update(Input, &Assets)` advances one deterministic tick at 60.0988 Hz.
Rendering reads game state without advancing it. Gameplay emits named sound
events; it has no dependency on the window or audio device. State uses world
coordinates and named Rust types, with no emulated registers or memory addresses.

## Editable assets

- `assets/cinematics/`: PNG tile atlases and editable JSON compositions/timelines
  for the mission map, helicopter deployment, attract story, stage victory,
  continue/loading screens and full ending. Audio cues retain original tick timing.
- `assets/fonts/hud.png`: original HUD and score glyphs.
- `assets/texture_phases.json` and `assets/textures/phases/`: recovered stage
  palette and tile streaming changes, including intermediate transition atlases.
- `assets/boss_patches.json`: original background tile compositions and collision for boss states.
- `assets/textures/hq_*` and `assets/textures/river_*`: original headquarters transitions, collapse and animated river artwork.
- `assets/textures/palette_cycles/`: PNG color swatches and JSON definitions for
  the timed scenery color cycles in all six stages.
- `assets/textures/stage_three_water/`: PNG water animation phases and a JSON manifest;
  the native scheduler preserves the original delay and partial graphics updates.
- `assets/textures/`: six background atlases, six transparent sprite atlases and
  the title image, all PNG. Each background atlas has four 256-tile palette rows
  (128×512 pixels); each sprite atlas has four 256-frame palette rows
  (128×1024 pixels, individual parts 8×16). Companion palette JSON files record
  the exported colors. Colors are baked into PNGs; palette JSON is provenance,
  not a separate runtime color override.
- `assets/levels/stage_01.json` through `stage_06.json`: complete tile maps,
  per-cell palette indices and collision, camera starts, spawns and boss triggers.
  Cells are row-major 8×8 tiles; collision values are 0 ground, 1 solid,
  2 water and 3 marsh. Marsh is traversable in stage four.
- `assets/levels/sprites.json`: sprite composition as named fields for part offsets,
  atlas indices, palettes and flips. Frame IDs are asset identifiers.
- `assets/levels/entity_specs.json`: named enemy families, health, armor, hitboxes,
  score and animation references. Attack timing lives in recovered state machines.
  Source IDs preserve provenance
  and join spawn records to definitions; they do not dispatch original code.
- `assets/animations.json`: named player, prisoner, pickup, projectile and infantry death clips.
- `assets/destruction.json`: recovered stage-specific prison rubble tile rectangles;
  each patch includes the recovered collision class independently of its artwork.
- `assets/audio/`: 43 original sound sequences exported to standard PCM WAV.
  `assets/audio.json` records names, hashes, sample counts and music loop points.
  WAVs are embedded at compile time by the sound module, so audio edits require
  rebuilding; changing loop metadata also requires updating its clip definition.

Maps and PNGs load at startup, so editing them requires only restarting the game.
`--assets DIRECTORY` overrides the graphics/level asset directory; embedded audio
continues to use the checked-in WAV files. Missing or inconsistent assets produce
an error before gameplay. No automatic extraction occurs during Cargo builds.

## Verification

```sh
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test
cargo run -- --headless 1200 --screenshot /tmp/jackal.png
cargo run -- --stage 6 --mute
```

Tests compare original reference pixels for helicopter deployment, scene transitions,
ending and HUD; thousands of ticks of camera, enemy, boss and projectile behavior;
prisoner states; all six initial maps; destruction; campaign flow and asset validation.
Scrolled-map tests additionally compare 54 scenes and 27 gun/turret placements across
all six stages, using exact pixels in the stable viewport and an actual native spawn.
Fixtures are produced by isolated historical execution, not copied from the new
implementation. Campaign tests accelerate progress using controlled damage and
camera placement; they do not establish an unassisted playthrough of every stage.

Headless mode starts at tick 60, then drives upward with gunfire and periodic
grenades after tick 240. It is a smoke-test input script, not an autoplayer.
`--stage 1..6` selects a stage directly. Screenshots are PNG.

## Evidence and fidelity boundary

Maps, sprite definitions, placements, health/armor/score tables, selected movement
and weapon rules, and sound sequences were recovered from the original program.
Reference fixtures come from the previous native static translation, not captures
from original hardware. Initial background comparisons exclude foreground objects
and the HUD. Scrolled geometry comparisons include isolated original guns/turrets,
fix the art phase, and exclude the bottom eight-row streaming strip; animation has
separate timing/artwork tests. Movement permits one pixel of cosmetic bob.

The new implementation replaces the original instruction translation entirely.
Presentation, camera, weapons and recovered actors have dedicated reference tests.
Shared actor allocation and priority, boss controllers, co-op target selection,
reinforcement waves and headquarters transitions have dedicated regression coverage.
Full-scene comparisons under arbitrary graphics upload contention still need
broader validation. Audio retains the historical
reference synthesizer's timing/filter approximation. The modern renderer preserves
sprite ordering and the shared part budget while omitting hardware scanline dropout.
A passing test establishes
only its recorded scenario, not complete hardware or whole-game parity.

`analysis/native-recovery.txt` and `analysis/native-entities.json` preserve detailed
recovery evidence. Older analysis files document the historical investigation,
including the unsuccessful REA/Ghidra deep-analysis attempt; they are not runtime
inputs. The previous translated implementation remains available in Git history.

Optional research exporters under `tools/reference/` retrieve the historical
reference revision into temporary directories. They reproduce assets and fixtures
without reintroducing its code or binary data into the game. See
[the export guide](tools/reference/README.md). Normal builds need none of these tools.
