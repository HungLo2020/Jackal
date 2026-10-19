use jackal_native::{
    math::Vec2,
    rescue_helicopter::{HelicopterContext, RescueHelicopter},
};
#[test]
fn rescue_helicopter_matches_original_positions_rotors_and_boarding_window() {
    let scenarios: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/rescue_helicopter.json")).unwrap();
    for scenario in scenarios.as_array().unwrap() {
        let name = scenario["name"].as_str().unwrap();
        let kind = scenario["kind"].as_u64().unwrap() as u8;
        let mut helicopter = RescueHelicopter::new(kind, Vec2::new(100., 100.)).unwrap();
        for f in scenario["frames"].as_array().unwrap() {
            let tick = f["tick"].as_u64().unwrap();
            let camera = Vec2::new(
                0.,
                if scenario["scroll"] == true {
                    -(tick as f32)
                } else {
                    0.
                },
            );
            let sounds = helicopter.tick(&HelicopterContext {
                camera,
                tick,
                carried_prisoners: if name == "right_with_passengers" && tick < 200 {
                    5
                } else {
                    0
                },
                forced_scroll: false,
            });
            assert_eq!(
                helicopter.alive(),
                f["alive"].as_bool().unwrap(),
                "{name}/{tick} alive"
            );
            if !helicopter.alive() {
                break;
            }
            assert_eq!(
                helicopter.position.x,
                f["x"].as_f64().unwrap() as f32,
                "{name}/{tick} x"
            );
            assert_eq!(
                helicopter.position.y - camera.y,
                f["y"].as_f64().unwrap() as f32,
                "{name}/{tick} y"
            );
            assert_eq!(
                helicopter.sprite,
                f["sprite"].as_u64().unwrap() as usize,
                "{name}/{tick} sprite"
            );
            assert_eq!(
                helicopter.dock_open,
                f["dock"].as_bool().unwrap(),
                "{name}/{tick} dock"
            );
            let expected: Vec<u8> = serde_json::from_value(f["sounds"].clone()).unwrap();
            assert_eq!(sounds, expected, "{name}/{tick} sounds");
        }
    }
}
