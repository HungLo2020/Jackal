# Native port fidelity audit

## Scrolled map alignment correction (2026-10-19)

The previous start-screen checks missed a cumulative map export error. Original
CD88-CD90 restarts every source page at subrow 2, omitting two 8-pixel rows.
The exporter retained those rows, while events correctly used 240-pixel pages.
This displaced background bases from turrets and other actors by another 16 pixels
per page. All six maps now export 30 rows per page, including corresponding palette
and collision rows. Initial camera origin is adjusted to preserve the start view.

The same audit corrected bottom-entry event Y from 224 to the original 239
(F17A-F18C), with top/bottom spawn tests at multiple event distances. It also
corrected actor palette composition: original bank1:81B3-81B9 ORs actor palette
bits with metasprite attributes. Replacing the descriptor palette caused incorrect
enemy grenade colors and infantry hit flashes. Explicit pixel tests exercise both
native sprite rendering paths.

New independent scene fixtures cover 54 scroll positions across all six stages,
including 27 gun/turret scenes. Exact pixel comparisons cover rows 0–231 after
settling the original video pipeline. The original bottom eight-row streaming
strip is explicitly outside these geometry assertions. Initial art/palette phases
are fixed for these comparisons; live scenery animation has separate timing and
artwork checks. An integration test additionally spawns the captured stationary
turret through native game logic and checks its original event/world anchor.

The scrolled comparisons also exposed missing stage-three water updates. A native
scheduler now preserves the original stage-entry countdown, alternating six-part
uploads, and interruption by the late palette transfer. Its 499-frame original
trace and 24 deduplicated PNG atlases cover the initial delay and eleven uploads.
The original exporter reproduces these assets and fixtures exactly. Loading,
pause, and other-stage behavior are tested separately. A further 2,368-frame
scrolling trace checks water patterns and colors while map transfers and timed
palette cycles compete for the graphics-update budget.


## Current restoration status (2026-10-18)

The helicopter deployment and original presentation flow are restored, including
both player counts, attract/story scenes, victory tally, loading/continue, ending
and harder-loop restart. The correction also restores native enemy/boss states,
prisoner release and rescue, reinforcement controllers, shared actor allocation,
co-op targeting, projectile/death art, headquarters/river transitions, recovered
patch collision and scene sprite priority. Reference regressions accompany these
changes. All changes are deliberately unstaged.

The evidence boundary remains explicit: controlled historical-program traces
and accelerated campaign tests do not prove every arbitrary-input playthrough
or every graphics-upload contention pattern. Audio uses the historical reference
synthesizer. The modern renderer retains sprite priority and the shared 64-part
budget but omits the hardware eight-sprites-per-scanline dropout. No game scene
is intentionally omitted. Historical findings and their subsequent corrections
follow below; later progress sections supersede earlier open-item lists.

This is a discrepancy checklist, not a claim of completed fidelity. A passing native
unit test proves only the behavior it asserts. Tests written against invented rules
cannot establish original-game parity.

Reference: historical static translation at
`e8fc3391d5b27fcd98b82fd27984141840449708`, using its original extracted data.
Original hardware timing/audio still needs independent validation. Runtime probes
below use isolated historical workspaces; no reference dispatcher belongs in the
native runtime.

## Game flow

| Feature | Recovered evidence | Rewrite discrepancy requiring correction |
| --- | --- | --- |
| Title idle / attract story | No-input boot enters state1 at tick0, state2 at385; story substates advance through4 and returns title at1527 | Static title without idle sequence |
| Mission introduction | Original has helicopter deployment/briefing sequence before gameplay | Generic 180-tick text panel removed original presentation |
| Stage victory | Stage1 controlled victory reaches state7 at relative tick0; substates1/2/3/4 at1/138/211/420; next stage load569, active601, with zero rescued | Generic180-tick overlay discards evacuation animation, tally, transitions |
| Final victory | Final-stage state7 immediately dispatches ending state9; substates1..7 span ticks2..1762, credits alternate8/9 until final10 at4907 | Generic ending text and Start accepted after180 ticks |
| Game over / Continue | Setting both life counters0 in active play produces state6 at0 and state8 at1; state8 still active after1800 idle ticks | Invented120-tick game-over delay and600-tick continue timeout |
| Continue controls | Needs original menu-choice and acceptance recovery | Select currently abandons game immediately; three-continue cap lacks cited evidence |
| Rescue bonus | C7E7..C81D decrements BCD rescued counters60/61 every8 global ticks and awards2000 each; when both reach0 sets delay96 | Invented pickup100, unload1000 and flat victory5000 must be removed; original animated subtotal required |
| HUD | Original bitmap glyph layout/counters and flashing behavior | Replaced by generic font/text; screenshot map tests deliberately exclude it |

Timing observations use new Game, Start only at tick120, run900 ticks, then controlled
state setup. Victory probes stream original boss area, keep protection counters255,
wait600 ticks, then set remaining-boss count0 and victory counterFE. These are
controlled state-transition probes, not unassisted playthroughs. Timings depend on
prior global frame where periodic countdown work is involved.

## Gameplay and world

- Enemy code explicitly identifies itself as authored. Infantry chase/patrol,
  generic horizontally oscillating vehicles, sine-wave boats and aimed shots replace
  independent original state machines. Entity families are a classification, not
  behavioral recovery. Each original handler/state needs a native equivalent and
  recorded input/state comparisons.
- Boss compositions/HP were recovered, but attack timing, movement, shields and
  introduction/defeat behavior remain independently authored. Passing campaign tests
  that directly damage bosses does not validate any of these patterns.
- F0/F1/F2 stage commands mean forced scroll, palette preparation and palette change.
  The rewrite consumes all commands but acts only on forced scroll. Renderer needs
  recovered palette transitions, including alternate scenery and boss visuals.
- Spawn event high bit requests priority allocation, including eviction when the
  original16-actor pool is full (F1EC); native unbounded vectors ignore this behavior.
  Allocation ordering and source variants affect subsequent states.
- Original rescue helicopters3D/3E land/wait based on passenger occupancy; generic
  flyover plus invisible unloading radius is not equivalent. Original type3F is a
  landing interaction, not an independent visible enemy.
- Every prison currently releases four prisoners at evenly spaced positions.
  Original prison variants and release timing/count/special-prisoner composition
  require reference recovery rather than a single universal value.
- Prison broken-building rectangles and infantry death sprites were recovered in
  the previous fix. Other destructible scenery still uses generic ground replacement;
  explosion animation and sprite priority/compositing need comparison too.
- Rocket range, explosion timing/patterns, player death spin, dropped prisoners,
  respawn placement, camera boundaries and two-player interaction need full traces.
  Existing80-frame walking trace is useful but not a comprehensive player test.
- Original enemy score index can increase when both players active (F1CF); one static
  score value per entity omits this rule.
- Existing map reference crops establish initial background accuracy only; they do
  not cover palette scripts, foreground priority, HUD, camera streaming over an entire
  level, object animation, or destructible background state.

## Completion criteria

No feature should be deleted or replaced merely to simplify native architecture.
Keep human-readable assets and named Rust state machines, but retain original
presentation, events, state transitions and timing. Record unresolved discrepancies
explicitly until reference comparisons pass. Fixtures should cover both player counts,
all stage transitions, full ending, continue choices, every independent enemy and
boss state, weapon upgrades, rescue sequences, palette changes and destruction.

## Additional controlled probes

`tests/fixtures/flow/original_states.txt`, `original_rescue_tally.txt` and
`original_continue.txt` preserve raw state traces. Probes are reproducible using
`tools/reference/flow_probe_main.rs` and `continue_probe_main.rs` as the historical
workspace main module (same temporary reference arrangement as other exporters).

The Continue limit is confirmed original behavior: three accepted restarts restore
five lives; the fourth game over remains in state6 before returning to title.
Select toggles original cursor50 between0 (Continue) and1 (End); it does not itself
leave the menu. Start confirms that choice. No600-tick countdown was observed.

The tally probe seeded carried counters54/55 with4/2 and rescued60/61 with BCD12/03.
Only rescued counters decreased during stage victory; carried counters stayed4/2
through next-stage initialization. Tally updates started at relative tick211,
completed tick299, and stage transitioned at568. This confirms that only previously
delivered prisoners earn the2000-point subtotal. Different rescue totals can affect
transition duration; captured zero-total visual timelines must support an extended
countdown hold, rather than cutting off large subtotals.

## Recovered boss actor coverage added during correction

`boss_states.rs` implements original tower18, flagged statue92 and guard31 actor
states from bank6 8E39..8EDC, 9592..96BB and AFEC..B102. Each is compared against
700 ticks of isolated original handler execution in `boss_sequences.json`, including
state, frame, firing timer and emitted actor types. Statue boss variants override
base type12 health with0x86 (six HP, explosive-only). Guard launchers reset their
health to0x83 on each opening. Tower attacks emit type39 shells; guard launchers
emit type23/24 enemies, not generic aimed bullets. Background patch IDs preserve
original opening/closing art.

These isolated tests do not establish complete boss encounter parity. Allocation
slots influence statue firing delay, original entity-pool capacity can suppress
spawns. Further recovery now covers boss1 armored tank movement, boss4 aircraft,
children23/24/42, and all four main headquarters actor types. These are native
state machines with reference fixtures, not sine-wave or generic-shot substitutes.

`boss_vehicle_sequences.json` compares three armored vehicle types over700 ticks;
`paratrooper_pod.json` compares the complete105-tick deployment. `hq_sequences.json`
compares700 ticks each of center4A, guard47, building4B and attached gun4F, including
building movement, beam cadence, aiming, recoil and shot timing. `hq_children.json`
checks guard missiles48 and building beams4C/4D through flight, expiry or original
world-bound removal. All fixture groups are reproducible with
`tools/reference/export_boss_sequences.py --check`. `export_boss_patches.py --check`
verifies the native background patch tiles and their original1x1/2x2/4x4 dimensions.

The headquarters controller now retains two missile guards and the attached gun.
Center death removes guards at24 ticks and starts the building at305 ticks;
the attached gun follows one tick later. A separate encounter test enforces this
sequence. Boss5 guards restore3HP every open tick and suppress tank deployment
when the shared count reaches the original per-guard limit (initially4, increasing
every64 openings). The paratrooper child AA=15 is a score index, not an activation
delay. HQ center-deployed tanks use score index0.

Remaining encounter validation: actual allocation slots and spawn-failure retries;
full two-player target selection; rank-dependent projectile lifetimes; boss death
palette/background and global explosion scripts; guard rubble compositing; all
phase-dependent collision shapes; successful original full fights at every rank.
HQ background damage thresholds and collapse timing are exposed for presentation
integration, but module tests alone do not establish full encounter parity. The
native port must not be described as fully recovered until those comparisons pass.

## Correction progress (2026-10-18)

The discrepancy list above records the defects found at the start of this pass.
It is retained as evidence, not as a statement that every row is still unfixed.

Recovered and integrated with reference tests:

- Original title/attract, mission map and helicopter deployment (both player
  counts), stage victory/tally, continue/loads, full ending and restart timings.
- Original sprite HUD, stage texture/palette transitions, intermediate streaming
  atlases. Backtracking does not undo consumed art commands.
- Camera and co-op boundaries: 3,200 reference frames, including limited reverse
  scrolling and forced-scroll control suppression. River-current movement: 216
  reference frames over all directions on clear and blocked terrain.
- Player weapon phases, range, impacts, upgrade blast waves and death/life timing.
- Roaming infantry, sentries, grenadiers, rotating/medium/armored tanks, boats,
  retracting launchers, truck infantry deployment and ambush actors have independent
  original execution fixtures. Remaining source coverage is still being audited.
- Prison variants release two/four/one prisoners with destruction/smoke delays and
  predecessor waits. Prisoner movement has four 400-tick reference traces. Jeep
  death drops up to four surviving passengers; unloading uses per-player timers,
  walking arrivals and the recovered 500-point award.
- Rescue helicopter parked/grace/spool/departure/return behavior: 2,307 reference
  ticks including sound and boarding availability.
- Boss actors, deployed vehicles/paratroopers, headquarters components, beams and
  missiles now use recovered state machines. Death timing has independent traces.
  Headquarters artwork and final collapse are undergoing integration.
- Original concealed bonuses, proximity-revealed mines, destructible background
  objects and co-op score-index advancement replace earlier generic behavior.
  Some of these currently have static-source/unit evidence only; that is weaker
  than full encounter execution evidence.

Open items at the initial integration checkpoint (see subsequent corrections):

- Complete every active source-ID route and child/controller behavior, especially
  environment formations/hazards and rescue reinforcement controllers.
- Restore original shared 16-actor allocation, slot order, priority eviction and
  spawn-failure behavior. Unbounded vectors and inferred slot numbers remain a
  fidelity defect and affect firing offsets, formations and explosion placement.
- Compare integrated encounter/presentation transitions, including headquarters
  collapse and moving-background streaming, rather than only isolated handlers.
- Validate full-stage collision, sprite ordering and co-op targeting. Current
  upload timelines reproduce controlled reference workloads, not every possible
  graphics-queue contention pattern.
- Independent original-hardware audio/timing validation remains outside the
  historical-translation reference evidence used for these fixtures.

### Shared actor execution and terrain correction

The shared 16-slot pool is now implemented with generational native handles,
original descending allocation, protected priority eviction and free-only child
allocation. Four hundred independent allocation probes match the original.
Boss controllers have 20 independent 1,800-tick traces, including full-pool
failures and retry timing. Managed boss tests cover all six encounters, tank
retry intervals and headquarters allocation/collapse. The integrated dispatcher
now executes actor slots from 15 down to 0; this replaces the earlier unbounded
vector/guessed-slot defect. Full-game parity still requires integrated comparisons.

Support aircraft 58/59/67 have six 600-tick original traces. Headquarters collapse,
rebuilding graphics, gun/beam children and final death are integrated. Earlier
"undergoing integration" and missing allocator notes above describe the earlier
checkpoint and are superseded by these results.

Patch collision is explicitly recovered, independently of the background art.
At DF93 the patch dimensions select the byte following its graphic tiles; DF99
stores that value, and E27D–E2D4 writes it into the separate two-bit collision
buffer at RAM 0300/0400. Thresholds used during initial level decoding must not
be reapplied to replacement graphics. Every exported prison/rubble patch has
collision value 1, so destroyed prison artwork remains blocked. Boss patches
contain values 0, 1 and 2. Both native JSON export tools preserve that byte and
TerrainView reads it directly. Tests verify blocked prison rubble and replacement
collision independent of the graphic tile index.

### Original sprite composition priority

Bank 1 8008–8059 submits logical actors 31 through 0. Metasprite parts are
submitted in their original table order; the score/lives HUD follows. Each render
advances the physical sprite origin by 17 slots and each submitted 8×16 part
advances by 49 slots modulo 64 (8013/81EC). The native renderer now queues parts
across actor categories, applies the same 64-part allocation budget and physical
priority, and includes HUD parts in that queue. This is ordinary native sprite
composition; no processor or PPU execution exists in the runtime.

`oam_order.json` independently captures all 64 possible origins with 32 distinct
actors; every native priority matches. The original visible Y is OAM Y+1, now
applied consistently rather than as a corpse-only workaround. The HUD selection
uses frame-clock bit 7 (128-tick blocks), correcting the earlier parity mistake.
New-game state 4 clears the rotation (observed at global tick 202 with Start at
120); separate probes found no rotation reset through stage clear, continue,
stage reload, or ending restart. Scanline overflow is not reproduced by the
modern renderer. Background-priority sprite flags were recovered in the follow-up below. Full
overlapping encounter raster comparisons remain separate validation work.

Boss damage now awards the original source score once on the death transition,
including original co-op index advancement and owner attribution. Tests cover
shields, armor, repeated damage and overlapping blast lobes. Boss and deployed
vehicle targeting now stores the slot-parity-selected player byte and advances
it only at recovered F574 call sites, rather than taking a shared global target.


### Airbase background-priority recovery

Frames 166, 167, 169 and 170 contain the only behind-background parts. These
belong to source51 formation aircraft created by source50, whose sole stage event
is stage4 distance1936. The JSON flag is now parsed and composed. Original
sprite priority is evaluated before background priority: an opaque pixel from a
higher-priority behind-background sprite suppresses lower-priority sprites even
where the background hides that pixel. The native renderer restores the saved
background in that case; transparent sprite pixels preserve lower-priority art.

The original video-data probe checks every background tile pixel in all four
palettes, proving color-index0 is exactly RGB black for the initial airbase
atlas and 40 consecutive snapshots through its complete palette/graphics change
at distance2707. Thus the PNG's black pixels provide an exact opacity mask for
the full relevant stage, without guessing from visual appearance. Reproduce
with `python3 tools/reference/check_background_priority.py --check`. Native
tests cover the overlapping-sprite masking rule and enforce the bounded
asset/stage/palette assumptions. All 11 renderer tests pass.

### October 2026 scrolling and palette regression audit

The original CD7A..CD90 map streamer skips two source tile rows from every map
page: each page covers 240 world pixels, not 256. The corrected exported geometry
now matches 54 settled original backgrounds across all six stages exactly in
rows 0–231, plus 27 original turret composites. The bottom eight rows are the
original nametable update strip; the fixtures retain those pixels but exclude
that documented streaming boundary. A separate test invokes native event
spawning and checks its stationary turret anchor against original world position.

The audit also recovered DDB0..DE26 timed palettes: four phases in page ranges
9–10, 7–8, 8–9, 9–10, 9–10, and 4–5 respectively. Native PNG swatches and the
scheduler match every timer request and delivered palette in all six scrolling
traces. This includes the 33-update cadence, next-update visibility, row-transfer
contention, and F1 preparation freeze. Original palette composition uses OR for
sprite attributes; replacing the base palette incorrectly was separately fixed.

Known bounded limitation: actor-generated graphics command bytes are not yet
accumulated in the palette scheduler's shared queue budget. Normal map-streaming
contention is recovered and tested; simultaneous patch-update contention can
still shift an animation request. This does not remove any palette phase.
