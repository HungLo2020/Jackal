mod audio;
mod game;
mod video;
use game::Game;
include!("cinematic_export_helpers.rs");
fn main() {
    let args: Vec<String> = std::env::args().collect();
    let root = &args[1];
    std::fs::create_dir_all(format!("{root}/tests/fixtures/cinematics")).unwrap();
    for players in 1..=2 {
        let mut g = Game::new(false).unwrap();
        let mut capture = Capture::new();
        for f in 0..1500 {
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
            let sounds = drain_sounds();
            if f == 119 {
                let mut title = Capture::new();
                title.frame(g.video.render());
                title.save(root, &format!("title_{players}p"), 119);
            }
            if f >= 120 {
                if g.memory[0x18] == 5 && g.memory[0x2c] == 3 {
                    println!("intro {players} control at{f}");
                    break;
                }
                capture.sound_commands(sounds);
                capture.frame(g.video.render());
                if [202, 420, 666, 751].contains(&f) {
                    g.video.screenshot(&format!(
                        "{root}/tests/fixtures/cinematics/intro_{players}p_tick_{}.png",
                        f - 120
                    ));
                }
            }
        }
        capture.save(root, &format!("intro_{players}p"), 120);
        // Capture an independent sprite-only HUD reference; background is transparent.
        while g.frame < 900 {
            g.update([0, 0]).unwrap();
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
    let mut g = Game::new(false).unwrap();
    let mut capture = Capture::new();
    for f in 0..1600 {
        g.update([0, 0]).unwrap();
        let sounds = drain_sounds();
        if (385..1528).contains(&f) {
            capture.sound_commands(sounds);
            capture.frame(g.video.render());
        }
    }
    capture.save(root, "attract", 385);
    let mut g = Game::new(false).unwrap();
    for f in 0..900 {
        g.update([if f == 120 { 8 } else { 0 }, 0]).unwrap();
    }
    g.memory[0x30] = 5;
    g.memory[0x18] = 5;
    g.memory[0x2c] = 1;
    g.memory[0x34] = 1;
    for _ in 0..90 {
        g.update([0, 0]).unwrap();
    }
    drain_sounds();
    g.memory[0x18] = 9;
    g.memory[0x19] = 0;
    let mut capture = Capture::new();
    for f in 0..6000 {
        g.update([0, 0]).unwrap();
        let sounds = drain_sounds();
        if g.memory[0x19] == 10 && f >= 5162 {
            println!("ending complete {f}");
            break;
        }
        capture.sound_commands(sounds);
        capture.frame(g.video.render());
        if [1, 600, 1200, 1800, 3000, 4800, 5161].contains(&f) {
            g.video.screenshot(&format!(
                "{root}/tests/fixtures/cinematics/ending_tick_{f}.png"
            ));
        }
    }
    capture.save(root, "ending", 0);
    for _ in 5163..6000 {
        g.update([0, 0]).unwrap();
    }
    drain_sounds();
    let mut capture = Capture::new();
    for f in 0..200 {
        g.update([if f == 0 { 8 } else { 0 }, 0]).unwrap();
        let sounds = drain_sounds();
        if g.memory[0x18] != 9 {
            break;
        }
        capture.sound_commands(sounds);
        capture.frame(g.video.render());
    }
    capture.save(root, "ending_restart", 6000);
    let mut capture = Capture::new();
    loop {
        capture.sound_commands(drain_sounds());
        capture.frame(g.video.render());
        g.update([0, 0]).unwrap();
        if g.memory[0x18] == 5 && g.memory[0x2c] == 3 {
            break;
        }
    }
    capture.save(root, "stage_load_01", 6084);
}
