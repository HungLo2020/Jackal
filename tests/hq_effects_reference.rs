use jackal_native::{
    hq_effects::{CollapseArt, CollapseExplosions},
    math::Vec2,
};
#[test]
fn collapse_art_and_sound_timeline_preserves_original_transition() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("assets");
    let art = CollapseArt::load(&root).unwrap();
    for (tick, command) in [(0, 0x17), (24, 0x59), (25, 0x27), (216, 0x48)] {
        assert_eq!(art.sounds(tick), [command]);
    }
    assert!(art.sounds(26).is_empty());
    let timeline: serde_json::Value = serde_json::from_slice(
        &std::fs::read(root.join("textures/hq_collapse/timeline.json")).unwrap(),
    )
    .unwrap();
    let frames = timeline["frames"].as_array().unwrap();
    assert_eq!(frames[24]["state"], 4);
    assert_eq!(frames[304]["state"], 5);
    assert_eq!(frames[305]["state"], 6);
    for phase in timeline["phases"].as_array().unwrap() {
        let tick = phase["tick_offset"].as_u64().unwrap() as u16;
        let (tiles, sprites) = art.textures(tick);
        assert_eq!((tiles.width, tiles.height), (128, 512));
        assert_eq!((sprites.width, sprites.height), (128, 1024));
    }
}
#[test]
fn explosion_cycle_advances_only_when_original_actor_slots_are_available() {
    let mut effects = CollapseExplosions::default();
    assert!(effects.spawn(25, 32, 0., 3).is_empty());
    assert!(effects.spawn(26, 33, 0., 3).is_empty());
    assert!(effects.spawn(26, 32, 0., 0).is_empty());
    assert_eq!(effects.spawn(26, 32, -100., 1), [Vec2::new(192., -68.)]);
    assert_eq!(
        effects.spawn(42, 48, -100., 4),
        [
            Vec2::new(256., -52.),
            Vec2::new(224., -36.),
            Vec2::new(320., -52.)
        ]
    );
    assert!(effects.spawn(281, 288, 0., 3).is_empty());
}

#[test]
fn collapse_explosion_positions_match_original_slot_spawns() {
    let source: serde_json::Value =
        serde_json::from_str(include_str!("../assets/textures/hq_collapse/timeline.json")).unwrap();
    let mut effects = CollapseExplosions::default();
    let mut previous = std::collections::BTreeSet::new();
    let mut checked = 0;
    for frame in source["frames"].as_array().unwrap() {
        let tick = frame["tick"].as_u64().unwrap() as u16;
        let entries = frame["explosions"].as_array().unwrap();
        let current: std::collections::BTreeSet<_> = entries
            .iter()
            .map(|e| e["slot"].as_u64().unwrap())
            .collect();
        if (26..281).contains(&tick) && frame["global_tick"].as_u64().unwrap() & 15 == 0 {
            let observed: Vec<_> = entries
                .iter()
                .rev()
                .filter(|e| !previous.contains(&e["slot"].as_u64().unwrap()))
                .map(|e| {
                    Vec2::new(
                        e["x"].as_f64().unwrap() as f32,
                        e["y"].as_f64().unwrap() as f32,
                    )
                })
                .collect();
            let expected = effects.spawn(
                tick,
                frame["global_tick"].as_u64().unwrap(),
                0.,
                observed.len(),
            );
            assert_eq!(expected, observed, "collapse tick {tick}");
            checked += expected.len();
        }
        previous = current;
    }
    assert!(
        checked >= 30,
        "original fixture must contain repeated explosions"
    );
}

#[test]
fn final_collapse_waits_for_all_208_updates_and_retains_explosion_cursor() {
    let source: serde_json::Value =
        serde_json::from_str(include_str!("../assets/textures/hq_final/timeline.json")).unwrap();
    let frames = source["frames"].as_array().unwrap();
    assert_eq!(frames.len(), 208);
    assert!(frames[..207].iter().all(|f| f["boss_alive"] != 0));
    assert_eq!(frames[207]["boss_alive"], 0);
    let center: serde_json::Value =
        serde_json::from_str(include_str!("../assets/textures/hq_collapse/timeline.json")).unwrap();
    let mut explosions = CollapseExplosions::default();
    for frame in center["frames"].as_array().unwrap() {
        explosions.spawn(
            frame["tick"].as_u64().unwrap() as u16,
            frame["global_tick"].as_u64().unwrap(),
            0.,
            3,
        );
    }
    // Original final-collapse first group: slots15,14,13,11,10 are occupied.
    let mut occupied = [true; 16];
    for slot in [12, 9, 8] {
        occupied[slot] = false;
    }
    let points = explosions.final_spawn(3, 64, Vec2::new(205., 0.), Vec2::new(371., 48.), occupied);
    let observed: Vec<_> = frames[3]["explosions"]
        .as_array()
        .unwrap()
        .iter()
        .rev()
        .map(|p| {
            Vec2::new(
                p["x"].as_f64().unwrap() as f32,
                p["y"].as_f64().unwrap() as f32,
            )
        })
        .collect();
    assert_eq!(points, observed);
}
