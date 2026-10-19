# HUD and stage artwork recovery

Evidence: historical reference commit
`e8fc3391d5b27fcd98b82fd27984141840449708`, native translated dispatch
annotations at `$F119–F15C`, `$DC7D`, `$D20B`, and `$DE5F–DF1B`, and controlled
reference execution from normal stage initialization.

The original gameplay HUD is an overlay at the bottom of the playfield. It
has no black header, weapon name or passenger count. Its original sprite
font is stored in `assets/fonts/hud.png`, extracted from the already decoded
native stage-one sprite atlas by `tools/export_hud.py`.

| Element | Player 1 | Player 2 |
| --- | --- | --- |
| Player label | (16, 201) | (160, 201) |
| Six-digit score | (40, 201) | (184, 201) |
| Reserve-life symbol | (44, 217) | (188, 217) |
| Reserve-life digit | (54, 217) | (198, 217) |

The reserve count is total lives minus the currently active jeep. In two-player
mode the original alternates the displayed HUD between players every 128 frames (bank 1, 8076: clock AND 80).
Reference frame 900 displays player 2. The native HUD retains this alternation.
A pixel-by-pixel test compares its entire 256 by 48 bottom region against
independent sprite-only captures for both player counts.

## Stage artwork changes

Earlier extraction named event `$F1` `palette_prepare` and `$F2`
`palette_change`. These names are incomplete descriptions. `$F1` calls
`$DC7D` and queues the stage's prepared palette data. `$F2` also starts a
streamed pattern upload, using a stage-specific entry from `$F15D`. The
routine at `$DE5F–DF1B` copies the replacement pattern data over several
frames and clears its active marker when finished. Merely recoloring the
initial atlas loses boss artwork; exporting four frames after the event also
captures incomplete artwork.

`tools/reference/export_palettes.py` now exports native PNG atlas variants
at every changed frame from the event through completion of that upload.
`assets/texture_phases.json` names the command distance and relative
`tick_offset` for each composition. Unchanged atlas files are shared between
consecutive entries. There is no streamed machine-data format in the game.

These timings were observed with the reference forced-scroll flag enabled,
player protection refreshed each frame and no attack buttons pressed. The
original upload routine checks available rendering-command space, so its
hardware scheduling can depend on rendering workload. The exported timings
faithfully retain this explicitly recorded scenario; they are not proof of
all possible original command-queue timing under arbitrary play.

The stage6 gate event (source73, handler9BC7) additionally requests graphics
command0x10. Its native PNG phases are recorded at distance3008 in
`texture_phases.json`; visible upload changes run offsets3–15. The event actor
survives only its initialization and release updates. Reproduce with
`tools/reference/export_gate_art.py`. Base palette exports preserve this
separately verified event timeline.

Stage6 river controllers (source68, FAF8/FB15) request graphics8 at creation
and age8, then alternate9/8 every8ticks. Visible uploads appear3ticks after
the actor request. `RiverArt` selects native PNG sequences for the early and
late palette contexts, keeping the rest of the atlas intact. No river object
survives to the final F2 graphics change. `tests/river_art_reference.rs`
compares all pixels at initial, alternating and repeat boundaries.
