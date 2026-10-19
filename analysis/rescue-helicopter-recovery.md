# Rescue helicopter evidence

Recovered from original bank 6 handlers BB1E (source 3C) and BB71–BD05
(sources 3D/3E), with fixed-bank movement F422/F47E and unloading E555/E997.
`src/rescue_helicopter.rs` expresses these states directly in Rust. Native
sprite PNGs supply frames 47–50; sound commands use the native audio manifest.

`tests/rescue_helicopter_reference.rs` compares every live tick's position,
rotor frame, unloading availability and sound commands against controlled
execution of the historical translation. Four scenarios cover both landing
orientations, carried passengers delaying takeoff, camera scrolling past the
landing, and the standalone flyover. Fixture regeneration is optional:
`python3 tools/reference/export_helicopter.py --check`.

Landing initializes the shared unloading direction (+1 left, -1 right), waits
128 countdown ticks plus the underflow/check tick, then a 256-tick boarding
grace period when no passengers remain. Spin-up lasts 63 updates, with rotor
speed/sound cadence increasing. The unloading flag clears on the first spin-up
update. A landing scrolling below screen Y304 skips spin-up and departs.
Departure travels upward 2.25 pixels/tick; crossing above the screen starts a
128-tick invisible delay, followed by a vertically flipped downward flyover
at 2.5 pixels/tick. The original fractional Y remainder survives this reset.
Standalone source 3C flies upward at 2.5 pixels/tick and freezes its velocity
under forced scrolling; landed helicopters do not freeze.

Source 3F has two roles: collision E555 identifies it as an unloading zone,
and bank 6 BD06 manages reinforcement spawns while a helicopter is available.
It must not be removed as a zone simply because it also controls reinforcements.
Delivery timers belong to players (initial 16, repeat 32), not the helicopter;
no delivery notification resets its grace timer.

Source3F reinforcement behavior is now `RescueZoneController`: every512ticks,
it consumes an attempt until the stage-index quota is reached. Stages2 and5
(one-based) spawn type59 with palette1; stage6 spawns type67 with palette2.
Other stages consume attempts without creating an actor. Full-pool rejection
does not retry. A closed dock advances to the release state and the zone
releases its actor slot on the following update. Reinforcement position starts
at worldX=zoneX-screenY, screenY=0; the support actor then applies its own
initialization logic.
