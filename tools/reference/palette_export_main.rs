mod audio;
mod game;
mod video;
use game::Game;
pub fn record_sound(_: u8) {}
fn main() {
    let args: Vec<String> = std::env::args().collect();
    let root = &args[1];
    std::fs::create_dir_all(format!("{root}/tests/fixtures/cinematics")).unwrap();
    for players in 1..=2 {
        let mut g = Game::new(false).unwrap();
        for f in 0..900 {
            g.update([
                if f == 120 {
                    8
                } else if players == 2 && f == 100 {
                    4
                } else {
                    0
                },
                0,
            ])
            .unwrap();
        }
        g.video.mask &= !8;
        let pixels = g.video.render();
        let backdrop = pixels[0];
        let mut hud = image::RgbaImage::new(256, 48);
        for y in 0..48 {
            for x in 0..256 {
                let c = pixels[(y + 192) * 256 + x];
                hud.put_pixel(
                    x as u32,
                    y as u32,
                    image::Rgba([
                        (c >> 16) as u8,
                        (c >> 8) as u8,
                        c as u8,
                        if c == backdrop { 0 } else { 255 },
                    ]),
                );
            }
        }
        hud.save(format!(
            "{root}/tests/fixtures/cinematics/hud_{players}p.png"
        ))
        .unwrap();
    }
    let mut stages = Vec::new();
    for stage in 0..6 {
        let mut g = Game::new(false).unwrap();
        for f in 0..900 {
            g.update([if f == 120 { 8 } else { 0 }, 0]).unwrap();
        }
        g.memory[0x30] = stage;
        g.memory[0x18] = 5;
        g.memory[0x2c] = 1;
        g.memory[0x34] = 1;
        for _ in 0..90 {
            g.update([0; 2]).unwrap();
        }
        let level: serde_json::Value = serde_json::from_slice(
            &std::fs::read(format!("{root}/assets/levels/stage_{:02}.json", stage + 1)).unwrap(),
        )
        .unwrap();
        let mut phases = Vec::new();
        for command in level["commands"].as_array().unwrap() {
            let name = command["command"].as_str().unwrap();
            if name == "forced_scroll" {
                continue;
            }
            let distance = command["distance"].as_u64().unwrap() as u32;
            let mut reached = false;
            for _ in 0..5000 {
                g.memory[0x52] = 255;
                g.memory[0x53] = 255;
                g.memory[0x3c] = 1;
                g.update([0, 0]).unwrap();
                let current = g.memory[0x43] as u32 * 240 + g.memory[0x44] as u32;
                if current >= distance {
                    reached = true;
                    println!(
                        "stage{} {} distance{} actual{} c7={} ba={}",
                        stage + 1,
                        name,
                        distance,
                        current,
                        g.memory[0xc7],
                        g.memory[0xba]
                    );
                    break;
                }
            }
            assert!(reached, "stage command not reached");
            let relative = format!("textures/phases/stage_{:02}/{name}", stage + 1);
            let directory = format!("{root}/assets/{relative}");
            std::fs::create_dir_all(&directory).unwrap();
            let scratch = format!("{root}/assets/.phase-export");
            std::fs::create_dir_all(&scratch).unwrap();
            let mut last_tiles = Vec::new();
            let mut last_sprites = Vec::new();
            let mut tiles_path = String::new();
            let mut sprites_path = String::new();
            let mut completed = false;
            for offset in 0..514 {
                g.video.export(&scratch, stage as usize + 1);
                let tiles =
                    std::fs::read(format!("{scratch}/stage_{:02}_tiles.png", stage + 1)).unwrap();
                let sprites =
                    std::fs::read(format!("{scratch}/stage_{:02}_sprites.png", stage + 1)).unwrap();
                let changed = tiles != last_tiles || sprites != last_sprites;
                if tiles != last_tiles {
                    tiles_path = format!("{relative}/frame_{offset:03}_tiles.png");
                    std::fs::write(format!("{root}/assets/{tiles_path}"), &tiles).unwrap();
                    last_tiles = tiles;
                }
                if sprites != last_sprites {
                    sprites_path = format!("{relative}/frame_{offset:03}_sprites.png");
                    std::fs::write(format!("{root}/assets/{sprites_path}"), &sprites).unwrap();
                    last_sprites = sprites;
                }
                if changed {
                    phases.push(serde_json::json!({"distance":distance,"tick_offset":offset,"command":name,"tiles":tiles_path,"sprites":sprites_path}));
                }
                if completed {
                    println!("stage{} {} fullyloadedafter{}", stage + 1, name, offset);
                    break;
                }
                if offset >= 2 && g.memory[0xba] == 0 {
                    completed = true;
                }
                g.memory[0x52] = 255;
                g.memory[0x3c] = 1;
                g.update([0, 0]).unwrap();
            }
            assert!(completed, "graphics stream did not finish");
            std::fs::remove_dir_all(&scratch).unwrap();
        }
        stages.push(phases);
    }
    // The separately recovered gate actor contributes an event-triggered stream.
    let gate_path = format!("{root}/assets/textures/hq_gate/timeline.json");
    if let Ok(bytes) = std::fs::read(gate_path) {
        let gate: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        for phase in gate["phases"].as_array().unwrap() {
            let mut phase = phase.clone();
            phase["distance"] = 3008.into();
            phase["command"] = "headquarters_gate".into();
            stages[5].push(phase);
        }
    }
    std::fs::write(
        format!("{root}/assets/texture_phases.json"),
        serde_json::to_string_pretty(&serde_json::json!({"stages":stages})).unwrap(),
    )
    .unwrap();
}
