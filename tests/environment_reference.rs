use jackal_native::{
    environment::{EnvironmentActor, EnvironmentContext},
    math::Vec2,
};
#[test]
fn formations_aircraft_and_hazards_match_original_actor_updates() {
    let scenarios: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/environment.json")).unwrap();
    for scenario in scenarios.as_array().unwrap() {
        let name = scenario["name"].as_str().unwrap();
        let mut actor = EnvironmentActor::new(
            scenario["kind"].as_u64().unwrap() as u8,
            Vec2::new(240., 200.),
            scenario["variant"].as_u64().unwrap() as u8,
            0,
        )
        .unwrap();
        let mut mask = 255;
        for frame in scenario["frames"].as_array().unwrap() {
            let tick = frame["tick"].as_u64().unwrap();
            if tick == 50 && name.ends_with("_killed") {
                actor.kill();
            }
            if tick == 50 && name == "wing_cascade" {
                mask = 191;
            }
            let events = actor.tick(
                &EnvironmentContext {
                    camera: Vec2::new(128., 0.),
                    tick,
                    page: scenario["page"].as_u64().unwrap() as u8,
                    forced_scroll: false,
                    target: Vec2::new(208., 160.),
                    free_slots: 15,
                },
                &mut mask,
            );
            assert_eq!(
                actor.alive,
                frame["alive"].as_bool().unwrap(),
                "{name}/{tick} alive"
            );
            assert_eq!(
                mask,
                frame["mask"].as_u64().unwrap() as u8,
                "{name}/{tick} mask"
            );
            if !actor.alive {
                break;
            }
            assert_eq!(
                actor.position.x,
                frame["x"].as_f64().unwrap() as f32,
                "{name}/{tick} x"
            );
            assert_eq!(
                actor.position.y,
                frame["y"].as_f64().unwrap() as f32,
                "{name}/{tick} y"
            );
            assert_eq!(
                actor.sprite,
                frame["sprite"].as_u64().unwrap() as usize,
                "{name}/{tick} sprite"
            );
            assert_eq!(
                actor.palette,
                frame["palette"].as_u64().unwrap() as u8,
                "{name}/{tick} palette"
            );
            assert_eq!(
                mask,
                frame["mask"].as_u64().unwrap() as u8,
                "{name}/{tick} mask"
            );
            let children = frame["children"].as_array().unwrap();
            assert_eq!(
                events.spawns.len() + events.shots.len(),
                children.len(),
                "{name}/{tick} child count"
            );
            for (child, original) in events.spawns.iter().zip(children) {
                assert_eq!(
                    child.source_type,
                    original["kind"].as_u64().unwrap() as u8,
                    "{name}/{tick} kind"
                );
                assert_eq!(
                    child.position,
                    Vec2::new(
                        original["x"].as_f64().unwrap() as f32,
                        original["y"].as_f64().unwrap() as f32
                    ),
                    "{name}/{tick} child position"
                );
                assert_eq!(
                    child.variant,
                    original["variant"].as_u64().unwrap() as u8,
                    "{name}/{tick} variant"
                );
                assert_eq!(
                    child.palette,
                    original["palette"].as_u64().unwrap() as u8,
                    "{name}/{tick} child palette"
                );
            }
            for ((direction, pattern), original) in events.shots.iter().zip(children) {
                assert_eq!(*pattern, 6);
                assert_eq!(
                    *direction,
                    original["direction"].as_u64().unwrap() as u8,
                    "{name}/{tick} shot direction"
                );
            }
            let sounds: Vec<u8> = serde_json::from_value(frame["sounds"].clone()).unwrap();
            assert_eq!(events.sounds, sounds, "{name}/{tick} sounds");
        }
    }
}
