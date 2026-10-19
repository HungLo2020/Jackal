//! Compare native state machines with isolated historical handler execution.
use jackal_native::{
    assets::Assets,
    enemy::{Behavior, Enemy},
    math::Vec2,
};
use std::sync::OnceLock;
fn assets() -> &'static Assets {
    static A: OnceLock<Assets> = OnceLock::new();
    A.get_or_init(|| {
        Assets::load(&std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("assets")).unwrap()
    })
}
#[test]
fn recovered_sentry_and_launcher_sequences_match_reference_for_ten_seconds() {
    let a = assets();
    let captures: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/enemy_sequences.json")).unwrap();
    for capture in captures.as_array().unwrap() {
        let id = capture["source_type"].as_u64().unwrap() as u8;
        let mut enemy = Enemy::new(
            a.entity(id),
            Vec2::new(80., 80.),
            if id <= 3 {
                Behavior::Infantry
            } else {
                Behavior::Turret
            },
        );
        for sample in capture["samples"].as_array().unwrap() {
            let tick = sample["tick"].as_u64().unwrap();
            let shots = enemy.tick(Vec2::new(80., 160.), &a.levels[0], a);
            assert_eq!(
                enemy.position,
                Vec2::new(80., 80.),
                "type {id} moved on tick {tick}"
            );
            assert_eq!(
                shots.len(),
                sample["shots"].as_u64().unwrap() as usize,
                "type {id}, tick {tick}: firing cadence"
            );
            {
                assert_eq!(
                    enemy.sprite(a),
                    Some(sample["sprite"].as_u64().unwrap() as u16),
                    "type {id}, tick {tick}: animation"
                );
            }
            if id <= 3 {
                assert_eq!(
                    enemy.palette_override(),
                    Some(sample["palette"].as_u64().unwrap() as u8),
                    "type {id}, tick {tick}:palette"
                );
            }
            assert_eq!(
                enemy.sprite_flips().0,
                sample["flip_x"].as_bool().unwrap(),
                "type {id}, tick {tick}: flip"
            );
        }
    }
}
#[test]
fn sentry_staggers_by_spawn_slot_and_does_not_fire_at_nearby_jeep() {
    let a = assets();
    let mut far = Enemy::new(a.entity(2), Vec2::new(80., 80.), Behavior::Infantry);
    far.configure_spawn(1, 0, 0);
    let mut near = far.clone();
    let mut first = None;
    for tick in 0..300 {
        if !far.tick(Vec2::new(80., 160.), &a.levels[0], a).is_empty() && first.is_none() {
            first = Some(tick);
        }
        assert!(near.tick(Vec2::new(80., 100.), &a.levels[0], a).is_empty());
    }
    assert_eq!(first, Some(16));
}
#[test]
fn heavy_launcher_only_accepts_damage_when_open() {
    let a = assets();
    let def = a.entity(46);
    let mut enemy = Enemy::new(def, Vec2::new(80., 80.), Behavior::Turret);
    enemy.tick(Vec2::new(80., 160.), &a.levels[0], a);
    assert!(!enemy.damage(5, true, def));
    for _ in 0..128 {
        enemy.tick(Vec2::new(80., 160.), &a.levels[0], a);
    }
    assert!(enemy.damage(5, true, def));
    assert_eq!(enemy.health, 59);
    for _ in 0..24 {
        enemy.tick(Vec2::new(80., 160.), &a.levels[0], a);
    }
    assert!(!enemy.damage(5, true, def));
}
#[test]
fn roaming_infantry_matches_twenty_seconds_of_original_patrol_aim_and_shoot_states() {
    struct Ground(u8);
    impl jackal_native::terrain::CollisionField for Ground {
        fn terrain_at(&self, _: Vec2) -> u8 {
            self.0
        }
    }
    let a = assets();
    let captures: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/roaming_infantry.json")).unwrap();
    for capture in captures.as_array().unwrap() {
        let id = capture["source_type"].as_u64().unwrap() as u8;
        let mut enemy = Enemy::new(a.entity(id), Vec2::new(80., 80.), Behavior::Infantry);
        for s in capture["samples"].as_array().unwrap() {
            let tick = s["tick"].as_u64().unwrap();
            let shots = enemy.tick(
                Vec2::new(80., 160.),
                &Ground(capture["terrain"].as_u64().unwrap() as u8),
                a,
            );
            assert_eq!(
                enemy.position,
                Vec2::new(
                    s["x"].as_f64().unwrap() as f32,
                    s["y"].as_f64().unwrap() as f32
                ),
                "type{id}tick{tick}:position"
            );
            assert_eq!(
                enemy.sprite(a),
                Some(s["sprite"].as_u64().unwrap() as u16),
                "type{id}tick{tick}:sprite"
            );
            assert_eq!(
                enemy.palette_override(),
                Some(s["palette"].as_u64().unwrap() as u8),
                "type{id}tick{tick}:palette"
            );
            assert_eq!(
                enemy.sprite_flips().0,
                s["flip_x"].as_bool().unwrap(),
                "type{id}tick{tick}:flip"
            );
            assert_eq!(
                shots.len(),
                s["shots"].as_u64().unwrap() as usize,
                "type{id}tick{tick}:shots"
            );
        }
    }
}
#[test]
fn rotating_turrets_match_original_aiming_arc_firing_muzzle_flash_and_recoil() {
    let a = assets();
    let captures: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/rotating_turrets.json")).unwrap();
    for c in captures.as_array().unwrap() {
        let id = c["source_type"].as_u64().unwrap() as u8;
        let stage = c["stage"].as_u64().unwrap() as usize;
        let target = Vec2::new(
            c["target"][0].as_f64().unwrap() as f32,
            c["target"][1].as_f64().unwrap() as f32,
        );
        let mut enemy = Enemy::new(a.entity(id), Vec2::new(80., 80.), Behavior::Vehicle);
        enemy.configure_spawn(0, stage, 0);
        for s in c["samples"].as_array().unwrap() {
            let tick = s["tick"].as_u64().unwrap();
            let shots = enemy.tick(target, &a.levels[stage], a);
            assert_eq!(
                enemy.position,
                Vec2::new(
                    s["x"].as_f64().unwrap() as f32,
                    s["y"].as_f64().unwrap() as f32
                ),
                "type{id} stage{stage} target{target:?} tick{tick}:position"
            );
            assert_eq!(
                enemy.sprite(a),
                Some(s["sprite"].as_u64().unwrap() as u16),
                "type{id} stage{stage} tick{tick}:sprite"
            );
            assert_eq!(
                enemy.palette_override(),
                Some(s["palette"].as_u64().unwrap() as u8),
                "type{id} tick{tick}:palette"
            );
            assert_eq!(
                enemy.sprite_flips(),
                (
                    s["flip_x"].as_bool().unwrap(),
                    s["flip_y"].as_bool().unwrap()
                ),
                "type{id} tick{tick}:flips"
            );
            assert_eq!(
                shots.len(),
                s["shots"].as_u64().unwrap() as usize,
                "type{id} tick{tick}:shots"
            );
        }
    }
}

fn validate_vehicle_fixture(fixture: &str, cooperative: bool) {
    struct Ground;
    impl jackal_native::terrain::CollisionField for Ground {
        fn terrain_at(&self, _: Vec2) -> u8 {
            0
        }
    }
    let a = assets();
    let captures: serde_json::Value = serde_json::from_str(fixture).unwrap();
    for c in captures.as_array().unwrap() {
        let id = c["source_type"].as_u64().unwrap() as u8;
        let stage = c["stage"].as_u64().unwrap() as usize;
        let target = Vec2::new(
            c["target"][0].as_f64().unwrap() as f32,
            c["target"][1].as_f64().unwrap() as f32,
        );
        let mut enemy = Enemy::new(a.entity(id), Vec2::new(80., 80.), Behavior::Vehicle);
        enemy.configure_spawn(0, stage, 0);
        if cooperative {
            enemy.set_players(
                [
                    target,
                    Vec2::new(
                        c["second_target"][0].as_f64().unwrap() as f32,
                        c["second_target"][1].as_f64().unwrap() as f32,
                    ),
                ],
                [true, true],
            );
        }
        if id == 30 {
            enemy.set_spawn_direction(10, (true, false));
        }
        for s in c["samples"].as_array().unwrap() {
            let tick = s["tick"].as_u64().unwrap();
            if cooperative {
                enemy.set_players(
                    [
                        target,
                        Vec2::new(
                            c["second_target"][0].as_f64().unwrap() as f32,
                            c["second_target"][1].as_f64().unwrap() as f32,
                        ),
                    ],
                    [true, tick < 300],
                );
            }
            let shots = enemy.tick(target, &Ground, a);
            let spawned = enemy.drain_spawns();
            assert_eq!(
                enemy.position,
                Vec2::new(
                    s["x"].as_f64().unwrap() as f32,
                    s["y"].as_f64().unwrap() as f32
                ),
                "type{id} stage{stage} target{target:?} tick{tick}:position"
            );
            assert_eq!(
                enemy.sprite(a),
                Some(s["sprite"].as_u64().unwrap() as u16),
                "type{id} stage{stage} tick{tick}:sprite"
            );
            assert_eq!(
                enemy.palette_override(),
                match id {
                    1..=6 => Some(s["palette"].as_u64().unwrap() as u8),
                    11 => Some(2),
                    14 => Some(1),
                    16 | 31 if tick >= 5 => Some(3),
                    18 => Some(s["palette"].as_u64().unwrap() as u8),
                    _ => None,
                },
                "vehicle palette"
            );
            assert_eq!(
                enemy.sprite_flips(),
                (
                    s["flip_x"].as_bool().unwrap(),
                    s["flip_y"].as_bool().unwrap()
                ),
                "type{id} tick{tick}:flips"
            );
            assert_eq!(
                shots.len() + spawned.len(),
                s["shots"].as_u64().unwrap() as usize,
                "type{id} tick{tick}:shots"
            );
        }
    }
}

#[test]
fn every_level_event_has_a_recovered_native_owner() {
    let a = assets();
    for level in &a.levels {
        for event in &level.events {
            let source = event.source_type;
            // These are owned by explicit boss, rescue, environment, background,
            // mine, terrain and pickup systems in Game::spawn_events.
            let separate_system = matches!(
                source,
                9 | 19
                    | 20
                    | 21
                    | 23
                    | 25
                    | 27
                    | 28
                    | 29
                    | 32
                    | 37
                    | 50
                    | 53
                    | 56
                    | 60
                    | 61
                    | 62
                    | 63
                    | 64
                    | 68
                    | 69
                    | 73
                    | 78
                    | 80
                    | 81
                    | 82
                    | 83
            );
            assert!(
                Enemy::supports_source(source) || separate_system,
                "{} event source{source} has no native handler",
                level.name
            );
        }
    }
}

#[test]
fn regular_vehicles_match_original_turning_motion_and_fire() {
    validate_vehicle_fixture(include_str!("fixtures/regular_vehicles.json"), false);
}
#[test]
fn cooperative_enemies_match_original_player_selection() {
    validate_vehicle_fixture(include_str!("fixtures/cooperative_enemies.json"), true);
}

#[test]
fn stopped_scripted_vehicles_enter_scoreless_death_on_original_actor_contact() {
    struct Ground(u8);
    impl jackal_native::terrain::CollisionField for Ground {
        fn terrain_at(&self, _: Vec2) -> u8 {
            self.0
        }
    }
    let a = assets();
    let captures: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/contact_deaths.json")).unwrap();
    for c in captures.as_array().unwrap() {
        let source = c["source"].as_u64().unwrap() as u8;
        let mut enemy = Enemy::new(a.entity(source), Vec2::new(128., 80.), Behavior::Vehicle);
        enemy.set_spawn_direction(10, (true, false));
        for sample in c["samples"].as_array().unwrap() {
            enemy.set_other_actors(vec![(7, enemy.position)]);
            enemy.tick(
                Vec2::new(160., 100.),
                &Ground(c["terrain"].as_u64().unwrap() as u8),
                a,
            );
            let triggered = enemy.take_death_trigger();
            assert_eq!(
                triggered,
                sample["triggered"].as_bool().unwrap(),
                "source {source} tick {}",
                sample["tick"]
            );
            assert_eq!(
                enemy.sprite(a),
                Some(sample["sprite"].as_u64().unwrap() as u16)
            );
            assert_eq!(
                enemy.position,
                Vec2::new(
                    sample["x"].as_f64().unwrap() as f32,
                    sample["y"].as_f64().unwrap() as f32
                )
            );
            if triggered {
                assert_eq!(enemy.score, 0);
            }
        }
        assert!(
            c["samples"].as_array().unwrap().last().unwrap()["triggered"]
                .as_bool()
                .unwrap()
        );
    }
}
