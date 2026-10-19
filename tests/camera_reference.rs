//! Logical positions and shared camera progression from the original game.
use jackal_native::{
    assets::Level, camera::CameraController, input::PlayerInput, math::Vec2, player::Player,
    renderer::Camera,
};
use serde::Deserialize;
#[derive(Deserialize)]
struct Scenario {
    name: String,
    two_players: bool,
    forced_scroll: bool,
    frames: Vec<Frame>,
}
#[derive(Deserialize)]
struct Frame {
    tick: u64,
    inputs: [u8; 2],
    x: [f32; 2],
    y: [f32; 2],
    camera_x: f32,
    distance: f32,
    backtrack: u8,
}
#[test]
fn logical_camera_and_jeep_positions_match_original_in_ten_scenarios() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let level: Level =
        serde_json::from_slice(&std::fs::read(root.join("assets/levels/stage_01.json")).unwrap())
            .unwrap();
    let scenarios: Vec<Scenario> = serde_json::from_slice(
        &std::fs::read(root.join("tests/fixtures/camera_motion.json")).unwrap(),
    )
    .unwrap();
    for scenario in scenarios {
        let mut camera = Camera {
            x: 0.,
            y: level.start_camera_y(),
        };
        let mut controller = CameraController::default();
        let mut players = [
            Player::new(Vec2::new(80., camera.y + 128.), true),
            Player::new(Vec2::new(176., camera.y + 128.), scenario.two_players),
        ];
        for frame in scenario.frames {
            if !scenario.forced_scroll {
                for (i, player) in players.iter_mut().enumerate() {
                    player.tick(
                        PlayerInput::from_buttons(frame.inputs[i]),
                        &level,
                        0,
                        990 + frame.tick,
                    );
                }
            }
            controller.update(
                &mut camera,
                &mut players,
                &level,
                false,
                scenario.forced_scroll,
            );
            let prefix = format!("{} tick{}", scenario.name, frame.tick);
            assert_eq!(camera.x, frame.camera_x, "{prefix} camera x");
            assert_eq!(
                240. + level.start_camera_y() - camera.y,
                frame.distance,
                "{prefix} progression"
            );
            assert_eq!(
                controller.backtrack_pixels(),
                frame.backtrack,
                "{prefix} reverse allowance"
            );
            for (i, player) in
                players
                    .iter()
                    .enumerate()
                    .take(if scenario.two_players { 2 } else { 1 })
            {
                assert_eq!(
                    player.position.x - camera.x,
                    frame.x[i],
                    "{prefix} player{i} x"
                );
                assert_eq!(
                    player.position.y - camera.y,
                    frame.y[i],
                    "{prefix} player{i} y"
                );
            }
        }
    }
}
