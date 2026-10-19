# Original scene alignment evidence

These PNGs come from the historical original translation at `e8fc339`, executed
by `tools/reference/export_scene_alignment.py`. They cover nine scroll distances
(480–2160) in each of the six stages. A subset also contains one actual original
turret/tank, using its live frame, position, palette and mirror flags. The remaining
sprites and HUD are removed by rebuilding the original sprite list, not by
painting over screenshots.

Scroll is stopped for three reference frames before capture so CPU coordinates
and the frame-based video pipeline settle. The comparisons require **exact pixel
equality** within rows 0–231. Rows 232–239 are the original nametable streaming
update strip and are outside this static placement test. They remain in the PNGs
for inspection; no percentage-match threshold hides their differences.

These are geometry fixtures. Each stage's initial pattern and palette memory is
restored only while rendering the sample, then the live state is restored before
execution continues. Nametable tiles/attributes and actor state are not replaced.
This isolates actor/base alignment from independently tested water animation and
palette changes. `fixed_initial_art_phase` records that constraint explicitly.

`tests/scene_alignment.rs` also verifies a stationary turret's observed screen
position against its event coordinate, then spawns that event through native
`Game::update` and checks the native world position.

Regenerate or verify:

```sh
python3 tools/reference/export_scene_alignment.py
python3 tools/reference/export_scene_alignment.py --check
cargo test --test scene_alignment
```

Set `JACKAL_ALIGNMENT_PREVIEWS=1` while running the test to write native stage-one
and stage-four comparison PNGs under `/tmp/native-stage_*_distance_*-turret.png`.
