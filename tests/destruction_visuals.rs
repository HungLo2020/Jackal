//! Regressions for actor death visuals, through actual projectile impacts.
use jackal_native::{
    assets::Assets,
    enemy::{Behavior, Enemy},
    game::{Game, GameEvent},
    input::Input,
    math::Vec2,
    player::WeaponLevel,
    sound::SoundEffect,
    weapon::{Projectile, ProjectileKind},
};

fn scene() -> (Assets, Game, Vec2) {
    let mut assets = Assets::load(&Assets::default_directory()).unwrap();
    assets.levels[0].collision.fill(0);
    let mut game = Game::new();
    game.start_new_game(1, &assets);
    game.enemies.clear();
    game.drain_events();
    let position = Vec2::new(200., game.camera.y + 160.);
    (assets, game, position)
}
fn impact(game: &mut Game, position: Vec2, kind: ProjectileKind) {
    game.projectiles.push(Projectile::new(
        position,
        Vec2::ZERO,
        kind,
        Some(0),
        1,
        WeaponLevel::Grenade,
    ));
}
#[test]
fn shot_soldiers_leave_noncombat_corpses_for_32_ticks_instead_of_explosions() {
    let (assets, mut game, position) = scene();
    for id in 1..=4 {
        game.start_stage(0, &assets);
        game.enemies.clear();
        game.drain_events();
        game.enemies
            .push(Enemy::new(assets.entity(id), position, Behavior::Infantry));
        let score = game.players[0].score;
        impact(&mut game, position, ProjectileKind::Bullet);
        game.update(Input::default(), &assets);
        assert!(
            game.enemies.is_empty(),
            "dead infantry must not collide or fire"
        );
        assert!(
            game.explosions.is_empty(),
            "gunfire must not turn soldiers into explosions"
        );
        assert_eq!(game.infantry_deaths.len(), 1);
        assert_eq!(game.infantry_deaths[0].frame(&assets), Some(24));
        let death_position = game.infantry_deaths[0].position;
        assert_eq!(game.players[0].score, score + assets.entity(id).score);
        let events = game.drain_events();
        assert!(events
            .iter()
            .any(|e| matches!(e, GameEvent::Effect(SoundEffect::InfantryDeath))));
        assert!(!events
            .iter()
            .any(|e| matches!(e, GameEvent::Effect(SoundEffect::Explosion))));
        for age in 1..32 {
            game.update(Input::default(), &assets);
            assert_eq!(game.infantry_deaths[0].age, age);
            assert_eq!(game.infantry_deaths[0].position, death_position);
            assert_eq!(game.infantry_deaths[0].frame(&assets), Some(24));
        }
        game.update(Input::default(), &assets);
        assert!(game.infantry_deaths.is_empty());
    }
}
#[test]
fn grenade_kills_keep_the_soldier_death_clip_in_addition_to_the_weapon_blast() {
    let (assets, mut game, position) = scene();
    game.enemies
        .push(Enemy::new(assets.entity(1), position, Behavior::Infantry));
    impact(&mut game, position, ProjectileKind::Grenade);
    game.update(Input::default(), &assets);
    assert_eq!(game.infantry_deaths.len(), 1);
    assert_eq!(game.explosions.len(), 1, "only the grenade itself explodes");
    game.start_stage(1, &assets);
    assert!(
        game.infantry_deaths.is_empty(),
        "corpses must not survive stage loading"
    );
}

#[test]
fn every_prison_variant_keeps_its_rubble_visible_after_the_blast() {
    use jackal_native::{
        renderer::Renderer,
        terrain::{CollisionField, TerrainView},
    };
    let (assets, mut game, _) = scene();
    for stage in 0..6 {
        for id in [0x13, 0x14, 0x15, 0x19, 0x1c, 0x1d] {
            game.start_stage(stage, &assets);
            game.enemies.clear();
            let position = Vec2::new(game.camera.x + 128., game.camera.y + 120.);
            game.enemies
                .push(Enemy::new(assets.entity(id), position, Behavior::Prison));
            impact(&mut game, position, ProjectileKind::Bullet);
            game.update(Input::default(), &assets);
            assert!(game.enemies.is_empty());
            assert!(
                game.prisoners.is_empty(),
                "prisoners wait until the destruction and smoke states finish"
            );
            assert_eq!(game.prison_releases.len(), 1);
            assert_eq!(game.terrain.patches.len(), 16);
            // Let the actual effect expire; remove released actors to inspect
            // unobscured rubble pixels independently of their current position.
            for _ in 0..24 {
                game.update(Input::default(), &assets);
            }
            assert!(game.explosions.is_empty());
            game.prisoners.clear();
            game.prison_releases.clear();
            game.boss_children.clear();
            let mut renderer = Renderer::new();
            let pixels = renderer.render(&game, &assets);
            let level = &assets.levels[stage];
            let atlas = &assets.tiles[stage];
            let terrain = TerrainView {
                level,
                changes: &game.terrain,
            };
            for (&index, patch) in &game.terrain.patches {
                let x = (index % level.width_tiles * 8) as f32;
                let y = (index / level.width_tiles * 8) as f32;
                // Original destruction records end in collision class 1 (DF93/E27D).
                assert_eq!(patch.collision, 1);
                assert_eq!(terrain.terrain_at(Vec2::new(x + 4., y + 4.)), 1);
                let (sx, sy) = game.camera.world_to_screen(x, y);
                let tile = usize::from(patch.tile) + usize::from(patch.palette) * 256;
                for dy in 0..8 {
                    for dx in 0..8 {
                        let at = ((tile / 16 * 8 + dy) * atlas.width + tile % 16 * 8 + dx) * 4;
                        let rgba = &atlas.rgba()[at..at + 4];
                        let expected = (u32::from(rgba[0]) << 16)
                            | (u32::from(rgba[1]) << 8)
                            | u32::from(rgba[2]);
                        assert_eq!(
                            pixels[(sy as usize + dy) * 256 + sx as usize + dx],
                            expected,
                            "stage{} prison{id:02x} rubble pixel",
                            stage + 1
                        );
                    }
                }
            }
        }
    }
}

#[test]
fn corpse_pixels_match_the_original_frame_including_palette_and_anchor() {
    use jackal_native::{enemy::InfantryDeath, renderer::Renderer};
    let (assets, mut game, _) = scene();
    game.infantry_deaths.push(InfantryDeath {
        actor_handle: None,
        position: Vec2::new(80., game.camera.y + 96.),
        age: 3,
    });
    let mut renderer = Renderer::new();
    let actual = renderer.render(&game, &assets);
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    let reference = image::open(root.join("infantry_death.png"))
        .unwrap()
        .to_rgb8();
    let metadata: serde_json::Value =
        serde_json::from_slice(&std::fs::read(root.join("infantry_death.json")).unwrap()).unwrap();
    assert_eq!(
        assets.animations["infantry_death"].frame_ticks,
        metadata["duration_ticks"].as_u64().unwrap() as u16
    );
    let atlas = &assets.sprites[0];
    let part = &assets.frames[24].parts[0];
    let tile = usize::from(part.tile) + 256;
    let mut compared = 0;
    for y in 0..16 {
        for x in 0..8 {
            let offset = ((tile / 16 * 16 + y) * atlas.width + tile % 16 * 8 + x) * 4;
            if atlas.rgba()[offset + 3] == 0 {
                continue;
            }
            let expected = reference.get_pixel((76 + x) as u32, (89 + y) as u32).0;
            let expected = (u32::from(expected[0]) << 16)
                | (u32::from(expected[1]) << 8)
                | u32::from(expected[2]);
            assert_eq!(actual[(89 + y) * 256 + 76 + x], expected);
            compared += 1;
        }
    }
    assert_eq!(compared, 66, "the complete visible corpse must be compared");
}
