# Jackal — native Rust port

Clone the repository and run from its directory:

```sh
cargo run
```

The translated Rust source and extracted game data are included in the repository.
No ROM file, Python installation, external emulator, or runtime CPU opcode
interpreter is required to build or play.
This is an **ahead-of-time native recompilation**, preserving the original game
logic and memory layout, rather than a hand-rewritten high-level game engine.
Rust implements the window, input, tile/sprite rendering, and sound synthesis.

The original program provides the menus, six stages, enemies, bosses, rescues,
weapon upgrades, scoring, two-player mode, continue system, and ending. The host
runs the recovered frame routine directly. It does not load arbitrary NES games.
Video and audio remain approximations of the console hardware; see verification
and limitations below before treating this as a fidelity-certified replacement.

## Controls

| Action | Player 1 | Player 2 |
|---|---|---|
| Move | Arrow keys | WASD |
| A / weapon | Z | G |
| B / weapon | X | H |
| Start | Enter | Y |
| Select | Tab | T |

Use Select on the title screen to choose two players. P pauses the native host,
R resets, and Escape exits. The window can be resized. `cargo run -- --mute`
disables audio. With no audio device, the game continues silently.

## Build requirements

Rust/Cargo, a desktop display, and the platform libraries needed by minifb and
rodio (on Linux: X11/Wayland and ALSA development libraries). A first build may
download Rust dependencies; game assets are already included.

## Included assets and native code

- `assets/game-data.bin`: 128 KiB of original banked data, including compressed
  graphics, maps, animation/event tables, music sequences, and sampled effects.
- `generated/native_game.rs`: the pretranslated Rust game logic, compiled directly
  by Cargo. No ROM translation happens during a normal build.
- `generated/translation.json`: original source identity, candidate entry count,
  and SHA-256 hashes of the included data and generated Rust.

The data keeps its original bank layout because game routines address tables by
bank and offset. Code and data were interleaved in the original program, so this
conservative data image also retains instruction bytes; these bytes are not
executed by a CPU interpreter. This is a banked data extraction, not separate PNG
and WAV exports. The compiled Rust routines decode graphics and drive the native
sound synthesizer. Both code and data are embedded in the executable.

The original `assets/jackal.nes`, intermediate analysis binaries, and build outputs
remain ignored by Git. They are not build or runtime dependencies.

## Reconstruction and evidence

- `analysis/rea-archive.json`: retained REA 6.7.0 archive inspection evidence.
- `analysis/rea-fixed-overview.json`: unsuccessful REA/Ghidra attempt and diagnostic.
- `analysis/rom.json`: independently measured ROM identity and layout.
- `analysis/findings.txt`: independent address-based findings, frame entry points,
  bank switching, state tables, stage/boss events, and test hooks.
- `tools/disassemble.py`: independent bank disassembly, e.g.
  `python3 tools/disassemble.py 7 c292 c2f4`.
- `tools/compile_game.py`: optional historical ROM-to-Rust lowering tool, adapted
  from the local MicroMages reconstruction. Only regenerating the translation
  requires Python and the original matching ROM; Cargo does not invoke it.
- `src/game.rs`: recovered frame scheduler, banked data access, and state helpers.
- `src/video.rs`: software graphics with story-scene split scrolling.
- `src/audio.rs`: pulse, triangle, noise, and original DMC samples.
- `src/window.rs`: desktop application identity and full-opacity X11 window hints.

Checked-in Rust and its translation manifest are under `generated/`. The original
translation pass lowered every legal-opcode
offset in each bank as a possible entry, including data that is never executed.
This conservative coverage handles the game's many computed dispatch tables;
the candidate count is **not** a count of recovered functions or proven code.
Unexpected unsupported transfers fail explicitly. There is no interpreter fallback.

REA's MCP open operation was rejected by client approval policy. Its installed
CLI successfully inspected the archive; deep Ghidra analysis reached import and
analysis but failed when the sandbox prohibited its local socket. Detailed game
logic findings therefore come from independent static decoding and native tests,
not a successful REA decompilation. No REA/client configuration was changed.

## Verification and remaining limits

```sh
cargo test --offline
cargo run --offline -- --headless 1200 --screenshot /tmp/jackal.ppm
```

Headless mode starts a one-player game at frame 120 and holds up/A after frame
500. Tests also use clearly marked state injection to reach stages and boss areas;
they are execution checks, not evidence of a complete human playthrough.
All 12 tests pass, covering both players, all six stages and boss areas, stage-clear
transitions, continuing, the final ending screen, and starting the next loop.
See `analysis/verification.txt` for the exact scope and test interventions.

Rendering uses complete-frame composition and original timing annotations for
the story split, not cycle-exact PPU simulation. Audio register changes are
applied at frame boundaries. Palette/emphasis and sprite overflow behavior are
approximations. Native captures are not pixel/audio differential comparisons
against original hardware. The initial sandbox blocked the display connection;
after access was enabled, a live X11 window was verified with application class
`JackalNative` and full opacity. The user confirmed the game runs. Audio-device
playback has not been independently checked.
