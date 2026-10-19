//! Regression boundaries come from static recovery or captured original-game fixtures.
//! These tests do not claim parity for the authored enemy/boss state machines.
use jackal_native::{
    assets::{Assets, Level},
    input::PlayerInput,
    math::Vec2,
    player::{Facing, Player, WeaponLevel},
    renderer::{Camera, Canvas},
    weapon::{self, ProjectileKind},
};
use serde::Deserialize;
use std::{path::PathBuf, sync::OnceLock};

fn assets() -> &'static Assets {
    static ASSETS: OnceLock<Assets> = OnceLock::new();
    ASSETS
        .get_or_init(|| Assets::load(&Assets::default_directory()).expect("exported native assets"))
}
fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}
fn clear_level() -> Level {
    let mut level = assets().levels[0].clone();
    level.collision.fill(0);
    level
}

#[test]
fn all_six_stages_resolve_their_original_events_and_artwork() {
    let a = assets();
    assert_eq!(a.levels.len(), 6);
    let original_event_counts = [74, 107, 133, 92, 138, 143];
    for (i, l) in a.levels.iter().enumerate() {
        assert_eq!(l.width_tiles, 64);
        assert_eq!(l.height_tiles, if i == 5 { 448 } else { 416 });
        assert_eq!(l.events.len(), original_event_counts[i]);
        for event in &l.events {
            let definition = a.entity(event.source_type);
            assert_eq!(
                event.kind,
                definition.kind,
                "stage {} event {}",
                i + 1,
                event.source_type
            );
            assert!(!definition.family.is_empty());
            for &frame in &definition.animation.frames {
                assert!((frame as usize) < a.frames.len());
            }
        }
    }
    assert_eq!(a.frames.len(), 256);
    // Original gun and grenade vulnerability metadata for the first boss tank.
    let tank = a.entity(0x0a);
    assert_eq!(tank.hit_points, 10);
    assert!(tank.grenade_only);
    assert_eq!(tank.score, 800);
}

#[test]
fn all_stage_map_crops_match_original_start_frames() {
    let a = assets();
    for (stage, x) in [0., 128., 68., 0., 144., 205.].into_iter().enumerate() {
        let level = &a.levels[stage];
        let packed: Vec<u16> = level
            .tiles
            .iter()
            .zip(&level.palettes)
            .map(|(&t, &p)| t + 256 * u16::from(p))
            .collect();
        let mut canvas = Canvas::default();
        canvas.tile_layer(
            &a.tiles[stage],
            &packed,
            level.width_tiles,
            8,
            Camera {
                x,
                y: level.height() - 256.,
            },
            None,
        );
        let expected = image::open(fixture(&format!("stage_{:02}_start.png", stage + 1)))
            .unwrap()
            .to_rgb8();
        assert_eq!(expected.dimensions(), (256, 240));
        let (mut equal, mut compared) = (0usize, 0usize);
        for y in 24..232 {
            for x in 8..256 {
                // HUD and the known player start rectangle are separate rendering layers.
                if (64..=96).contains(&x) && (108..=148).contains(&y) {
                    continue;
                }
                let rgb = expected.get_pixel(x as u32, y as u32).0;
                let color =
                    (u32::from(rgb[0]) << 16) | (u32::from(rgb[1]) << 8) | u32::from(rgb[2]);
                equal += usize::from(canvas.pixels()[y * 256 + x] == color);
                compared += 1;
            }
        }
        let ratio = equal as f64 / compared as f64;
        assert!(
            ratio >= 0.97,
            "stage {} background match {:.3}%",
            stage + 1,
            ratio * 100.
        );
    }
}

#[derive(Deserialize)]
struct Motion {
    frame: u64,
    input: u8,
    x: f32,
    y: f32,
}
#[test]
fn player_positions_follow_original_controller_recording() {
    let level = &assets().levels[0];
    let camera_y = level.height() - 256.;
    let samples: Vec<Motion> =
        serde_json::from_slice(&std::fs::read(fixture("player_motion.json")).unwrap()).unwrap();
    assert_eq!(samples.len(), 80);
    let mut player = Player::new(Vec2::new(80., camera_y + 128.), true);
    for sample in samples {
        player.tick(
            PlayerInput::from_buttons(sample.input),
            level,
            0,
            sample.frame,
        );
        assert_eq!(
            player.position.x, sample.x,
            "x at original frame {}",
            sample.frame
        );
        assert!(
            (player.position.y - camera_y - sample.y).abs() <= 1.,
            "y at original frame {}: native {}, original {}",
            sample.frame,
            player.position.y - camera_y,
            sample.y
        );
    }
}

#[test]
fn diagonal_speed_and_marsh_follow_recovered_movement_rules() {
    let mut level = clear_level();
    let start = Vec2::new(100., 100.);
    let mut p = Player::new(start, true);
    let input = PlayerInput {
        up: true,
        right: true,
        ..Default::default()
    };
    for t in 0..20 {
        p.tick(input, &level, 0, t);
    }
    assert_eq!(p.position, Vec2::new(120., 80.));
    level.collision.fill(3);
    let mut marsh = Player::new(start, true);
    for t in 0..20 {
        marsh.tick(input, &level, 3, t);
    }
    assert_eq!(marsh.position, Vec2::new(110., 90.));
    let mut blocked = Player::new(start, true);
    blocked.tick(input, &level, 0, 0);
    assert_eq!(blocked.position, start);
}

#[test]
fn repeated_movement_cannot_cross_a_one_tile_wall() {
    let mut level = clear_level();
    for row in 0..level.height_tiles {
        level.collision[row * level.width_tiles + 16] = 1;
    }
    let mut p = Player::new(Vec2::new(100., 100.), true);
    for t in 0..600 {
        p.tick(
            PlayerInput {
                right: true,
                ..Default::default()
            },
            &level,
            0,
            t,
        );
    }
    // E2E8 probes the candidate center +8: x120 would sample blocked x128.
    assert_eq!(p.position.x, 119., "east leading edge must stop at x127");
}

#[test]
fn machine_gun_uses_press_edges_global_32_tick_repeat_and_three_slots() {
    let mut p = Player::new(Vec2::new(100., 100.), true);
    let input = PlayerInput {
        gun: true,
        ..Default::default()
    };
    let first = weapon::fire(&p, 0, input, 5, &[]);
    assert_eq!(first.len(), 1);
    assert_eq!(first[0].kind, ProjectileKind::Bullet);
    assert_eq!(first[0].velocity, Vec2::new(0., -6.));
    assert_eq!(first[0].remaining, 16);
    p.previous_input = input;
    for t in 6..32 {
        assert!(
            weapon::fire(&p, 0, input, t, &[]).is_empty(),
            "early repeat at {t}"
        );
    }
    assert_eq!(weapon::fire(&p, 0, input, 32, &[]).len(), 1);
    let three = vec![first[0].clone(); 3];
    p.previous_input = PlayerInput::default();
    assert!(weapon::fire(&p, 0, input, 33, &three).is_empty());
    // Another player's slots do not constrain this jeep.
    assert_eq!(weapon::fire(&p, 1, input, 33, &three).len(), 1);
    let mut shot = first[0].clone();
    for _ in 0..22 {
        shot.tick();
    }
    assert_eq!(shot.remaining, 0);
    assert_eq!(shot.position, Vec2::new(100., 10.));
    // Collision is swept: a small target between the endpoints still gets hit.
    let mut crossing = first[0].clone();
    crossing.tick(); // Initialization frame does not move.
    crossing.tick();
    assert!(crossing.touches(Vec2::new(100., 97.), 0.5));
    assert!(!crossing.touches(Vec2::new(102., 97.), 0.5));
}

#[test]
fn grenades_require_a_new_press_and_one_slot_with_original_travel() {
    let mut p = Player::new(Vec2::new(100., 100.), true);
    p.requested_facing = Facing::NorthEast;
    let input = PlayerInput {
        grenade: true,
        ..Default::default()
    };
    let shots = weapon::fire(&p, 0, input, 1, &[]);
    assert_eq!(shots.len(), 1);
    assert_eq!(shots[0].kind, ProjectileKind::Grenade);
    p.previous_input = input;
    assert!(weapon::fire(&p, 0, input, 32, &[]).is_empty());
    p.previous_input = PlayerInput::default();
    assert!(weapon::fire(&p, 0, input, 33, &shots).is_empty());
    assert_eq!(weapon::fire(&p, 1, input, 33, &shots).len(), 1);
    let mut grenade = shots[0].clone();
    for _ in 0..41 {
        grenade.tick();
    }
    assert!(!grenade.is_flying());
    assert_ne!(grenade.remaining, 0); // Explosion continues to reserve the weapon slot.
    assert_eq!(grenade.position, Vec2::new(160., 40.));
    p.requested_facing = Facing::East;
    let mut cardinal = weapon::fire(&p, 0, input, 34, &[]).remove(0);
    for _ in 0..41 {
        cardinal.tick();
    }
    assert_eq!(cardinal.position, Vec2::new(180., 100.));
}

#[test]
fn extra_lives_start_at_twenty_thousand_then_every_fifty_thousand() {
    let mut p = Player::new(Vec2::ZERO, true);
    p.add_score(19_999);
    assert_eq!(p.lives, 5);
    p.add_score(1);
    assert_eq!(p.lives, 6);
    p.add_score(49_999);
    assert_eq!(p.lives, 6);
    p.add_score(1);
    assert_eq!(p.lives, 7);
    p.add_score(1_000_000);
    assert_eq!(p.lives, 9);
}

#[test]
fn health_and_protection_are_independent_for_two_players() {
    let mut a = Player::new(Vec2::new(80., 128.), true);
    let b = Player::new(Vec2::new(176., 128.), true);
    assert!(!a.hit(), "new jeep has original160-tick protection");
    let level = clear_level();
    for t in 0..160 {
        a.tick(PlayerInput::default(), &level, 0, t);
    }
    assert!(a.hit());
    assert_eq!(
        a.lives, 5,
        "life decrements after the original death sequence"
    );
    assert_eq!(b.lives, 5);
    assert!(!a.hit(), "respawning jeep cannot lose another life");
    assert_eq!(b.invulnerable, 160);
}

#[test]
fn rescue_delivery_thresholds_and_special_prisoners_upgrade_weapons() {
    let mut p = Player::new(Vec2::new(80., 128.), true);
    for _ in 0..16 {
        assert!(p.rescue(false));
    }
    assert_eq!(
        p.passengers, 16,
        "original collection counter has no eight-passenger cap"
    );
    for delivered in 1..=15 {
        assert!(p.deliver_one());
        let expected = match delivered {
            1..=3 => WeaponLevel::Grenade,
            4..=8 => WeaponLevel::Rocket,
            9..=14 => WeaponLevel::TwoWay,
            _ => WeaponLevel::FourWay,
        };
        assert_eq!(p.weapon, expected, "delivered {delivered}");
    }
    assert_eq!(p.passengers, 1);
    let mut special = Player::new(Vec2::ZERO, true);
    assert!(special.rescue(true));
    assert_eq!(special.weapon, WeaponLevel::Rocket);
    special.invulnerable = 0;
    assert!(special.hit());
    assert_eq!(special.weapon, WeaponLevel::Grenade);
    assert_eq!(special.passengers, 0);
}

#[test]
fn recovered_armor_rejects_gunfire_and_two_grenades_destroy_ten_hp_tank() {
    use jackal_native::enemy::{Behavior, Enemy};
    let def = assets().entity(0x0a);
    let mut tank = Enemy::new(def, Vec2::new(100., 100.), Behavior::Vehicle);
    assert!(!tank.damage(1, false, def));
    assert_eq!(tank.health, 10);
    assert!(tank.damage(5, true, def));
    assert!(tank.alive);
    assert_eq!(tank.health, 5);
    assert!(tank.damage(5, true, def));
    assert!(!tank.alive);
}
