# Headquarters collapse

Original bank 6 B739–B790 transitions the destroyed center turret through a
24-tick explosion, then the fixed-bank FC6D–FDAF global collapse. That collapse
must remain interactive: players and their projectiles continue to update.
`src/hq_effects.rs` supplies native PNG artwork and timed sound cues; it does
not replace gameplay with a movie.

Center death tick 0 starts sound 0x17. Tick 24 deletes non-HQ enemies and
stops the previous sound (0x59). Tick 25 starts collapse sound 0x27 and the
streamed replacement graphics (command 0x11). Tick 216 issues sound 0x48.
The original FC6D countdown underflows from zero, runs 256 updates, then
advances the palette commands 0x3d through 0x54. Center state advances at
304 and the headquarters building spawns at 305. The building then requests graphics commands 0x0a–0x0d while moving;
these are a separate native animation, not part of the collapse fade.

FCB3 generates three explosion actors on global clock multiples of 16 while
the collapse countdown is running. FD64 supplies thirteen repeating world-X /
screen-Y positions; the position cursor advances only when an actor slot is
available. Hidden explosion groups during the fade still advance the cursor
before being removed in the same tick; the cursor persists into final collapse. `CollapseExplosions` accepts the caller's free slot count to retain
this behavior. A separate native explosion animation renders these actors.

`tools/reference/export_hq_effects.py` regenerates PNG atlases and the JSON
timeline through controlled execution of the historic reference. The JSON
also retains observed explosion actors and sound commands for regression
checks. Upload timings are measured for that controlled scenario; arbitrary
render queue saturation can affect the original console's upload latency.


Final building death runs FC45 for 208 updates, leaving the boss alive through
update206 and clearing it on207. Its replacement artwork appears over offsets
2–11. Final explosion X positions are relative to the building's last screen
position. Native slot scanning preserves the original increment/carry rules;
regression checks compare the first observed group including occupied slots.

Moving building commands are requested every four ticks. Their uploads become
visible in three pieces, overlapping the following request. Eight directional
transition timelines plus the initial transition preserve this overlap. The
renderer selects them using the previous two requested phases, current phase,
and time since the request. Actors remain fully native and interactive.
