# Formation aircraft and cycling hazards

`src/environment.rs` ports original bank6 A684 (formation controller), A6EF
(formation planes), 943A (cycling hazard), and9539 (splash children). It uses
native positions, sprite frame IDs and explicit counters. No translated CPU
state or runtime binary data is required.

The formation requests eight aircraft at24-tick intervals, beginning with
variant128/palette2 and then64..1/palette1. Shared destruction bits reproduce
cascading aircraft deaths. Aircraft travel upward1.5pixels/tick, with the
original alternate-clock firing masks and delayed wing-frame changes. Direct
hits use4HP and1500points; cascade deaths do not themselves award hit points.
The first allocation is retried on failure; a later failed allocation ends the
formation controller, matching A6E7.

Hazards cycle world X positions240,304,368. A32-tick warning precedes24active
updates, whose sprite palette cycles. Page7 uses sprite95 and splash offsets
±64; other pages use sprite96 and offsets±56. Active collision flags0x7a map
to12x54 half extents, while inactive flags0xf0 are noncolliding. Splash sprites
97/98 blink for16updates and release their actor slot afterward.

`src/game/environment.rs` integrates allocation order and native bullet
pattern6 (speed1.75, base lifetime136, delay7, sprite16, palette3), preserving
same-frame initialization when a child occupies a lower slot than its parent.

`tests/environment_reference.rs` verifies1949original actor updates in ten
scenarios: formation spawning, multiple aircraft variants, direct kills and
shared-mask cascade, both hazard layouts, and splash lifetime. Checks include
positions, frames, palettes, sounds, spawned child positions/variants and shot
direction. Optional reproduction: `python3 tools/reference/export_environment.py --check`.
