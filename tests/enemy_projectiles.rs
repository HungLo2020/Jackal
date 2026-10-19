use jackal_native::{math::Vec2, terrain::CollisionField, weapon::Projectile};
struct Ground(u8);
impl CollisionField for Ground {
    fn terrain_at(&self, _: Vec2) -> u8 {
        self.0
    }
    fn traversable(&self, _: Vec2, _: bool) -> bool {
        true
    }
}
fn validate_projectiles(fixture: &str, cooperative: bool) {
    let fixture: serde_json::Value = serde_json::from_str(fixture).unwrap();
    for capture in fixture.as_array().unwrap() {
        let id = capture["source_type"].as_u64().unwrap();
        let variant = capture["variant"].as_u64().unwrap_or(if id == 52 {
            8
        } else if id == 57 {
            2
        } else {
            1
        }) as u8;
        let mut shot = if id == 52 {
            Projectile::enemy_grenade_pattern(Vec2::new(256., 160.), 8, u16::from(variant))
        } else if id == 54 {
            Projectile::sentry_bullet(Vec2::new(256., 160.), Vec2::new(0., 1.), 64)
        } else if id == 55 {
            Projectile::enemy_bomb(Vec2::new(256., 160.), 8, variant == 0, 0)
        } else {
            Projectile::enemy_missile_pattern(Vec2::new(256., 160.), 24, variant, 112)
        };
        if cooperative {
            shot.set_players([Vec2::new(256., 240.), Vec2::new(192., 64.)], [true, true]);
        }
        for sample in capture["samples"].as_array().unwrap() {
            let tick = sample["tick"].as_u64().unwrap();
            if cooperative {
                shot.set_players(
                    [Vec2::new(256., 240.), Vec2::new(192., 64.)],
                    [true, tick < 80],
                );
            }
            shot.tick_enemy(
                Vec2::new(256., 240.),
                &Ground(capture["terrain"].as_u64().unwrap_or(0) as u8),
            );
            let expected = Vec2::new(
                sample["x"].as_f64().unwrap() as f32,
                sample["y"].as_f64().unwrap() as f32,
            );
            assert_eq!(
                shot.position, expected,
                "projectile{id} tick{tick}:position"
            );
            if id != 57 {
                assert_eq!(
                    shot.remaining > 0,
                    sample["type"].as_u64().unwrap() != 0,
                    "projectile{id}tick{tick} lifetime"
                );
            }
            if sample["type"].as_u64().unwrap() != 0 {
                if let Some(palette) = sample["palette"].as_u64() {
                    assert_eq!(
                        shot.enemy_palette(),
                        palette as u8,
                        "projectile{id}tick{tick}palette"
                    );
                }
                assert_eq!(
                    shot.enemy_sprite().unwrap_or(0),
                    sample["sprite"].as_u64().unwrap() as u16,
                    "projectile{id} tick{tick}:sprite"
                );
                let flips = sample["flips"].as_u64().unwrap();
                assert_eq!(
                    shot.sprite_flips(),
                    (flips & 64 != 0, flips & 128 != 0),
                    "projectile{id} tick{tick}:flips"
                );
            }
        }
    }
}

#[test]
fn grenade_and_homing_missile_follow_original_position_animation_and_lifetime() {
    validate_projectiles(include_str!("fixtures/enemy_projectiles.json"), false);
}
#[test]
fn cooperative_projectiles_preserve_original_target_switching() {
    validate_projectiles(include_str!("fixtures/cooperative_projectiles.json"), true);
}

#[test]
fn stage_five_final_approach_turret_bullets_pass_walls_only_at_original_boundary() {
    for (stage, page, scroll, config, passes) in [
        (4, 11, 192, 12, true),
        (4, 11, 255, 13, true),
        (4, 11, 191, 12, false),
        (4, 10, 255, 12, false),
        (3, 11, 192, 12, false),
        (4, 11, 192, 11, false),
    ] {
        let mut shot = Projectile::enemy_pattern(Vec2::new(80., 80.), Vec2::new(1., 0.), 96, 1, 16);
        shot.configure_enemy_terrain(stage, config);
        let palette = shot.enemy_palette();
        shot.configure_enemy_map(stage, page, scroll);
        for _ in 0..5 {
            shot.tick_enemy(Vec2::ZERO, &Ground(1));
        }
        assert_eq!(
            shot.position.x > 82.,
            passes,
            "stage {stage} page {page} scroll {scroll} config {config}"
        );
        assert_eq!(shot.enemy_palette(), palette);
    }
}
