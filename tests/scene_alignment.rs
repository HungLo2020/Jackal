//! Compare native map and turret placement with independently executed original scenes.
use jackal_native::{
    assets::Assets,
    renderer::{Camera, Renderer},
};
use serde::Deserialize;
#[derive(Deserialize)]
struct Actor {
    source_type: u8,
    frame: u16,
    x: f32,
    y: f32,
    palette: u8,
    flips: u8,
}
#[derive(Deserialize)]
struct Sample {
    stage: usize,
    distance: f32,
    camera_x: f32,
    name: String,
    actor: Option<Actor>,
}
fn expected(name: &str) -> Vec<u32> {
    image::open(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/scene_alignment")
            .join(name),
    )
    .unwrap()
    .to_rgb8()
    .pixels()
    .map(|p| u32::from(p[0]) << 16 | u32::from(p[1]) << 8 | u32::from(p[2]))
    .collect()
}
#[test]
fn original_scrolled_maps_and_turrets_keep_their_shared_world_alignment() {
    let assets = Assets::load(&Assets::default_directory()).unwrap();
    let samples: Vec<Sample> =
        serde_json::from_str(include_str!("fixtures/scene_alignment/samples.json")).unwrap();
    // Original streaming reuses the bottommost eight scanlines as its update
    // strip. They are deliberately outside this static map/actor alignment ROI.
    // Pattern and palette memory use each stage's fixed initial art phase,
    // isolating geometry from separately verified live texture/palette changes.
    let mut failures = Vec::new();
    for sample in samples {
        let level = &assets.levels[sample.stage];
        let camera = Camera {
            x: sample.camera_x,
            y: level.start_camera_y() - (sample.distance - 240.),
        };
        let tiles: Vec<_> = level
            .tiles
            .iter()
            .zip(&level.palettes)
            .map(|(&tile, &palette)| tile + u16::from(palette) * 256)
            .collect();
        let mut renderer = Renderer::new();
        renderer.canvas.tile_layer(
            &assets.tiles[sample.stage],
            &tiles,
            level.width_tiles,
            8,
            camera,
            None,
        );
        let background = expected(&format!("{}_background.png", sample.name));
        let differences = renderer
            .canvas
            .pixels()
            .iter()
            .zip(&background)
            .take(232 * 256)
            .filter(|(a, b)| a != b)
            .count();
        if differences > 0 {
            failures.push(format!(
                "{} background: {differences} mismatched pixels",
                sample.name
            ));
        }
        if let Some(actor) = sample.actor {
            renderer.sprite(
                &assets,
                sample.stage,
                actor.frame,
                camera.x + actor.x,
                camera.y + actor.y,
                camera,
                Some(actor.palette),
                actor.flips & 64 != 0,
                actor.flips & 128 != 0,
            );
            if std::env::var_os("JACKAL_ALIGNMENT_PREVIEWS").is_some()
                && ["stage_4_distance_1040", "stage_1_distance_480"].contains(&sample.name.as_str())
            {
                let bytes: Vec<u8> = renderer
                    .canvas
                    .pixels()
                    .iter()
                    .flat_map(|&p| [(p >> 16) as u8, (p >> 8) as u8, p as u8])
                    .collect();
                image::save_buffer(
                    format!("/tmp/native-{}-turret.png", sample.name),
                    &bytes,
                    256,
                    240,
                    image::ColorType::Rgb8,
                )
                .unwrap();
            }
            if actor.source_type == 12 {
                let event = level
                    .events
                    .iter()
                    .find(|event| {
                        event.source_type == 12
                            && event.x == camera.x + actor.x
                            && (sample.distance - event.distance - actor.y).abs() < 0.01
                    })
                    .expect("stationary turret must align with its original event coordinate");
                assert_eq!(
                    camera.y + actor.y,
                    level.start_camera_y() - (event.distance - 240.)
                );
            }
            let scene = expected(&format!("{}_turret.png", sample.name));
            let differences = renderer
                .canvas
                .pixels()
                .iter()
                .zip(&scene)
                .take(232 * 256)
                .filter(|(a, b)| a != b)
                .count();
            if differences > 0 {
                failures.push(format!(
                    "{} turret: {differences} mismatched pixels",
                    sample.name
                ));
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn stationary_turret_event_spawns_at_the_original_background_anchor() {
    use jackal_native::{game::Game, input::Input, math::Vec2};
    let mut assets = Assets::load(&Assets::default_directory()).unwrap();
    let samples: Vec<Sample> =
        serde_json::from_str(include_str!("fixtures/scene_alignment/samples.json")).unwrap();
    let sample = samples
        .iter()
        .find(|sample| sample.actor.as_ref().is_some_and(|a| a.source_type == 12))
        .unwrap();
    let actor = sample.actor.as_ref().unwrap();
    let level = &mut assets.levels[sample.stage];
    level.events.retain(|event| {
        event.source_type == 12
            && event.x == sample.camera_x + actor.x
            && (sample.distance - event.distance - actor.y).abs() < 0.01
    });
    assert_eq!(level.events.len(), 1);
    let camera = Camera {
        x: sample.camera_x,
        y: level.start_camera_y() - (sample.distance - 240.),
    };
    let mut game = Game::new();
    game.start_new_game(1, &assets);
    game.start_stage(sample.stage, &assets);
    game.camera = camera;
    game.players[0].position = Vec2::new(camera.x + 128., camera.y + 128.);
    game.update(Input::default(), &assets);
    let turret = game
        .enemies
        .iter()
        .find(|enemy| enemy.definition == 12)
        .unwrap();
    assert_eq!(
        turret.position,
        Vec2::new(camera.x + actor.x, camera.y + actor.y)
    );
}
