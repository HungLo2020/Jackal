use jackal_native::{
    math::Vec2,
    player::{Facing, WeaponLevel},
    weapon::{Projectile, ProjectileKind},
};
#[test]
fn all_eight_directions_and_all_weapon_phases_match_original_frames() {
    let fixture: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/player_weapons.json")).unwrap();
    for capture in fixture.as_array().unwrap() {
        let name = capture["weapon"].as_str().unwrap();
        let facing = capture["facing"].as_u64().unwrap() as usize - 1;
        let direction = [
            Facing::North,
            Facing::NorthEast,
            Facing::East,
            Facing::SouthEast,
            Facing::South,
            Facing::SouthWest,
            Facing::West,
            Facing::NorthWest,
        ][facing]
            .vector();
        let diagonal = direction.x != 0. && direction.y != 0.;
        let position = Vec2::new(128., 128.);
        let mut shot = match name {
            "bullet" => Projectile::player(
                position,
                Vec2::new(0., -6.),
                ProjectileKind::Bullet,
                0,
                16,
                WeaponLevel::Grenade,
            ),
            "grenade" => Projectile::player(
                position,
                direction * if diagonal { 1.5 } else { 2. },
                ProjectileKind::Grenade,
                0,
                40,
                WeaponLevel::Grenade,
            ),
            "rocket" => Projectile::player(
                position,
                direction * if diagonal { 2.75 } else { 4. },
                ProjectileKind::Rocket,
                0,
                22,
                WeaponLevel::FourWay,
            ),
            _ => Projectile::blast_wave(position, 0, name == "wave_vertical"),
        };
        for sample in capture["samples"].as_array().unwrap() {
            let tick = sample["tick"].as_u64().unwrap();
            let detonates = shot.tick_player(tick);
            assert_eq!(
                shot.position,
                Vec2::new(
                    sample["x"].as_f64().unwrap() as f32,
                    sample["y"].as_f64().unwrap() as f32
                ),
                "{name} facing{facing} tick{tick}:position"
            );
            assert_eq!(
                shot.player_sprite().unwrap_or(0),
                sample["sprite"].as_u64().unwrap() as u16,
                "{name} facing{facing} tick{tick}:frame"
            );
            let flips = sample["flips"].as_u64().unwrap();
            assert_eq!(
                shot.sprite_flips(),
                (flips & 64 != 0, flips & 128 != 0),
                "{name} facing{facing} tick{tick}:flip"
            );
            assert_eq!(
                shot.remaining == 0,
                sample["state"].as_u64().unwrap() == 0,
                "{name} facing{facing} tick{tick}:lifetime"
            );
            if name == "rocket" && detonates {
                assert_eq!(shot.blast_waves().len(), 2);
                assert_eq!(sample["horizontal_wave"], 9);
                assert_eq!(sample["vertical_wave"], 9);
            }
        }
    }
}
#[test]
fn impact_stops_bullets_but_preserves_secondary_slot_during_explosion() {
    let mut shot = Projectile::player(
        Vec2::new(80., 80.),
        Vec2::new(0., -4.),
        ProjectileKind::Rocket,
        0,
        22,
        WeaponLevel::Rocket,
    );
    shot.tick_player(0);
    shot.impact();
    assert!(!shot.is_flying());
    assert!(!shot.is_player_blast());
    assert!(shot.tick_player(1));
    assert!(shot.is_player_blast());
    for t in 2..25 {
        shot.tick_player(t);
        assert_ne!(shot.remaining, 0);
    }
    shot.tick_player(25);
    assert_eq!(shot.remaining, 0);
    let mut bullet = Projectile::player(
        Vec2::new(80., 80.),
        Vec2::new(0., -6.),
        ProjectileKind::Bullet,
        0,
        16,
        WeaponLevel::Grenade,
    );
    bullet.tick_player(0);
    bullet.impact();
    assert!(!bullet.can_hit_enemies());
    assert_eq!(bullet.player_sprite(), Some(17));
    for t in 1..=5 {
        bullet.tick_player(t);
    }
    assert_eq!(bullet.remaining, 0);
    assert_eq!(bullet.position, Vec2::new(80., 80.));
}
