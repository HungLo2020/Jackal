# Optional reference exports

These tools are for reproducibility, not game builds. They retrieve the historical
static translation and banked data from local Git revision
`e8fc3391d5b27fcd98b82fd27984141840449708` into temporary directories. A shallow
checkout must obtain that revision before running them. The native runtime never
loads or links these reference files.

```sh
python3 tools/reference/export_graphics.py --check
python3 tools/reference/export_audio.py --check
python3 tools/reference/export_destruction.py --check
```

`--check` regenerates and compares checked-in assets. Without it, the exporters
replace their outputs. Graphics include PNG atlases, palette JSON, title art and
test fixtures; audio includes all 43 WAV files and the manifest. Cargo dependencies
must be cached for offline reference builds; audio also offers `--online`.
See [audio details](AUDIO_EXPORT.md).

The map/entity extractor is a separate static Python tool:

```sh
python3 tools/extract_levels.py --help
```

It requires an explicitly supplied original PRG data file. This file is a research
input only, and is not retained in the asset tree. It writes level/entity/sprite
JSON and provenance analysis. Native named animation clips are authored separately.
Reference exports establish repeatability against the old implementation; they
are not a hardware accuracy certification.

## Fidelity regression exports

The correction pass adds independent execution fixtures for presentation and
individual gameplay systems. Each exporter below supports `--check`; normal game
builds and tests use the checked-in PNG/JSON/WAV outputs directly.

| Coverage | Exporters |
| --- | --- |
| Mission map, deployment, ending and stage transitions | `export_cinematics.py`, `export_victory.py` |
| Stage palettes, headquarters gate/collapse and river animation | `export_palettes.py`, `export_gate_art.py`, `export_hq_effects.py`, `export_river_art.py`, `export_stage_three_water.py`, `export_palette_timing.py` |
| Camera, jeep movement, river current and weapons | `export_movement.py`, `export_current.py`, `export_player_weapons.py` |
| Enemy states, projectiles and deaths | `export_enemy_sequences.py`, `export_enemy_projectiles.py`, `export_enemy_deaths.py`, `export_contact_deaths.py` |
| Co-op targeting | `export_cooperative_enemies.py`, `export_cooperative_projectiles.py` |
| Boss states, controllers and collision patches | `export_boss_sequences.py`, `export_boss_controllers.py`, `export_boss_patches.py`, `export_patch_collision.py` |
| Prisoners, rescue helicopters and reinforcements | `export_prisoners.py`, `export_helicopter.py`, `export_rescue_zone.py` |
| Environment hazards, formations and mines | `export_environment.py`, `export_mines.py` |
| Shared actor capacity and sprite priority | `export_actor_pool.py`, `export_oam_order.py` |

The matching Rust drivers in this directory run only in temporary reference
workspaces. Fixtures under `tests/fixtures/` record original outputs, while tests
exercise the new Rust state machines against those outputs. Controlled traces
establish the covered state transitions; they are not a substitute for integrated
playthrough and rendering comparisons.

`export_stage_three_water.py` captures the original stage entry, alternating
six-chunk water uploads, and interruption by the later palette upload. The runtime
uses a compact manifest and deduplicated PNG atlases; the 499-frame register and
pixel evidence lives separately in `tests/fixtures/stage_three_water.json`.

`export_palette_timing.py` records stage-three loader counters, natural palette
requests and displayed palette bytes, plus isolated queue-capacity boundary
cases. This independently checks the cycle period and one-frame display delay.

`export_palette_cycles.py --check` reproduces native PNG color swatches for all
six timed background palette animations and the complete scrolling reference
trace. `export_palette_timing.py --check` independently verifies stage entry,
queue gates, the 33-update request cadence, and delivery one update later.
The native scheduler retains a pending tile-row upload, so a row transfer also
postpones palette requests when scrolling stops immediately after a boundary.
The source palette bands have four distinct colors, checked when loading, so
composition with the early river and water PNG animations preserves each tile's
original color indices. Sprite colors and the universal background color are
validated unchanged. Scripted palette preparation takes over the palette and
freezes timed requests on the following update.

The recovered row-upload budget covers normal scrolling. Actor-generated render
commands (for example, simultaneous destruction patches) are not yet accumulated
in this shared budget; the scheduler accepts an additional byte budget and tests
the original queue threshold explicitly. This is a remaining contention-timing
boundary, rather than an omitted palette animation.

`export_water_moving.py` captures 2,368 playable updates while scrolling through
stage three. Its test checks shared map-upload contention, alternating water
patterns, and delivered palette-cycle colors together against the original.
