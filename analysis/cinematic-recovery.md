# Original presentation recovery

Evidence source: deterministic execution of historical reference commit
`e8fc3391d5b27fcd98b82fd27984141840449708`, whose translated original game
and extracted data are restored only in a temporary research workspace.
The normal Rust executable does not load or execute this reference.

## Observed opening sequence

Start was pressed at reference frame 120. With one player selected:

| Reference frame | Observed transition |
| --- | --- |
| 120 | Start accepted; flashing title selection |
| 201 | Title transition completes |
| 202 | Mission map and original good-luck message |
| 347 | Mission map finishes |
| 348 | Stage loading begins |
| 379 | Helicopter sequence initialization |
| 380 | Helicopter arrival |
| 581 | Helicopter reaches drop position |
| 616–704 | Jeep drop and deployment |
| 752 | Player control begins |

The exported one-player introduction contains frames 120–751 (632 ticks).
The two-player variant reaches player control at frame 768 and contains
648 ticks, including the second jeep drop. Start at frame 300 or 500, or
held continuously after frame 120, did not skip or shorten the sequence.
Only the first stage of a new game uses this helicopter sequence. Later
stages use the original short loading transition.

Recorded sound-driver entry calls at `$c36b` are selection effect `$17`
at introductory tick 0 and briefing music `$34` at tick 83. No separate
rotor sound command was observed in this bounded trace. The stage music
starts on return to gameplay.

## Other restored presentation

The title selection is exported separately for one and two players. The
attract story is captured from reference frames 385–1527 (1,143 ticks).
The final ending reaches its final screen at tick 4,906, then waits 256
additional ticks before accepting held Start (tick 5,162). The exported
ending includes this wait. Start leads to a further 84-tick hold before the
32-tick first-stage load. It does not automatically restart the campaign. Its recorded sound
cues include the victory theme, text effects, stop command `$59`, and the
ending theme at tick 1,762.

Stage victory, game-over, Continue selection and loading transitions are
recovered separately by `tools/reference/export_victory.py` (see that
exporter's evidence and dynamic-score handling). These screens contain
variable scores; game code retains responsibility for those values and
for input-driven choices.

## Native representation and verification

`assets/cinematics/manifest.json` enumerates named clips. Each clip has an
ordinary PNG tile atlas and a JSON timeline of tile compositions, durations,
and sound commands. An 8-pixel tile is only an artwork subdivision, not a
hardware register or instruction. Repeated compositions and tiles are shared
to avoid decoding thousands of full-screen images. `src/cinematic.rs`
validates and renders these compositions directly through the native canvas.

`tools/reference/export_cinematics.py` reproduces title, introduction,
attract and ending assets. Fifteen independently rendered reference PNGs
cover mission map, helicopter arrival, jeep deployment, departure, and
multiple ending scenes. `tests/cinematic_reference.rs` compares every pixel
at these checkpoints and tests exact durations, final-frame holding and
sound-cue timing. The asset loader rejects invalid durations, runs, tile
indices, and commands absent from the audio manifest.

These checks establish presentation equivalence at sampled frames and
recovered timeline lengths. They do not establish parity for unrelated
native enemy AI, weapons, bosses or other gameplay systems.
