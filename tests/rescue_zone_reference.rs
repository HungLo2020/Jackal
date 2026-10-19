use jackal_native::{math::Vec2, rescue_helicopter::RescueZoneController};

#[test]
fn rescue_zone_reinforcements_and_closure_match_original_all_stages() {
    let cases: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/rescue_zone.json")).unwrap();
    for case in cases.as_array().unwrap() {
        let stage = case["stage"].as_u64().unwrap() as u8;
        let end = case["end"].as_u64().unwrap();
        let close = case["close"].as_u64();
        let mut controller = RescueZoneController::default();
        let mut spawns = Vec::new();
        for tick in 0..=end.min(3199) {
            let spawn = controller.tick(
                Vec2::new(240., 200.),
                Vec2::ZERO,
                stage,
                close.is_none_or(|close| tick < close),
            );
            if let Some(spawn) = spawn {
                spawns.push(serde_json::json!({"tick":tick,"source_type":spawn.source_type,"palette":spawn.palette,"x":spawn.position.x as i32,"y":spawn.position.y as i32}));
            }
            assert_eq!(controller.alive(), tick < end, "stage {stage}, tick {tick}");
        }
        assert_eq!(
            serde_json::Value::Array(spawns),
            case["spawns"],
            "stage {stage}"
        );
    }
}
