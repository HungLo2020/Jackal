use jackal_native::river_art::RiverArt;
#[test]
fn river_tiles_change_at_original_eight_tick_requests_after_upload_latency() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("assets");
    let art = RiverArt::load(&root).unwrap();
    for (late, name) in [(false, "early"), (true, "late")] {
        for (age, scene, offset) in [
            (0, "initial", 0),
            (2, "initial", 0),
            (3, "initial", 2),
            (16, "initial", 2),
            (18, "0_1", 0),
            (19, "0_1", 2),
            (24, "0_1", 2),
            (27, "1_0", 2),
            (35, "0_1", 2),
        ] {
            let timeline: serde_json::Value = serde_json::from_slice(
                &std::fs::read(root.join(format!("textures/river_{name}_{scene}/timeline.json")))
                    .unwrap(),
            )
            .unwrap();
            let phase = timeline["phases"]
                .as_array()
                .unwrap()
                .iter()
                .rev()
                .find(|p| p["tick_offset"].as_u64().unwrap() <= offset)
                .unwrap();
            let (tiles, sprites) = art.textures(late, age);
            for (texture, key) in [(tiles, "tiles"), (sprites, "sprites")] {
                let expected = image::open(root.join(phase[key].as_str().unwrap()))
                    .unwrap()
                    .to_rgba8();
                assert_eq!(
                    texture.rgba(),
                    expected.as_raw(),
                    "river {name} age{age} {key}"
                );
            }
        }
    }
}
