mod audio;
mod game;
mod video;
use game::Game;
static PALETTE_QUEUE: std::sync::atomic::AtomicU8 = std::sync::atomic::AtomicU8::new(255);
fn main() {
    let root = std::env::args().nth(1).unwrap();
    std::fs::create_dir_all(format!("{root}/assets/textures/palette_cycles")).unwrap();
    std::fs::create_dir_all(format!("{root}/tests/fixtures")).unwrap();
    let bounds = [[9, 11], [7, 9], [8, 10], [9, 11], [9, 11], [4, 6]];
    let mut manifests = vec![];
    let mut traces = vec![];
    for stage in 0..6 {
        let mut g = Game::new(false).unwrap();
        for f in 0..900 {
            g.update([if f == 120 { 8 } else { 0 }, 0]).unwrap();
        }
        g.memory[0x30] = stage as u8;
        g.memory[0x18] = 5;
        g.memory[0x2c] = 1;
        g.memory[0x34] = 1;
        let mut frames = vec![];
        let mut phases = vec![None; 4];
        let mut previous = 0;
        let mut waiting = None;
        let mut base = vec![];
        for tick in 0..3300 {
            let mode = g.memory[0x2c];
            let before = [
                g.memory[0xc7],
                g.memory[0xc4],
                g.memory[0x43],
                g.memory[0x21],
            ];
            g.memory[0x52] = 255;
            g.memory[0x53] = 255;
            if mode == 3 {
                g.memory[0x3c] = 1;
            }
            PALETTE_QUEUE.store(255, std::sync::atomic::Ordering::Relaxed);
            g.update([0; 2]).unwrap();
            let palette = g.video.palette_rgb();
            if base.is_empty() && g.memory[0x2c] == 3 {
                base = palette.clone();
            }
            let phase = g.memory[0xc4];
            if phase != previous {
                waiting = Some((tick + 2, previous as usize));
                previous = phase;
            }
            if let Some((deadline, p)) = waiting {
                if tick == deadline {
                    phases[p] = Some(palette.clone());
                    waiting = None;
                }
            }
            frames.push(serde_json::json!({"tick":tick,"mode":mode,"before":before,"after":[g.memory[0xc7],phase,g.memory[0x43],g.memory[0x21]],"palette":palette,"queue":PALETTE_QUEUE.load(std::sync::atomic::Ordering::Relaxed),"distance":u32::from(g.memory[0x43])*240+u32::from(g.memory[0x44])}));
            if g.memory[0x43] >= bounds[stage][1] + 1 {
                break;
            }
        }
        assert!(
            phases.iter().all(Option::is_some),
            "stage {} missing phases",
            stage + 1
        );
        let phases: Vec<Vec<u32>> = phases.into_iter().map(Option::unwrap).collect();
        let mut image = image::RgbImage::new(32, 5);
        for (y, colors) in std::iter::once(&base).chain(phases.iter()).enumerate() {
            for (x, &c) in colors.iter().enumerate() {
                image.put_pixel(
                    x as u32,
                    y as u32,
                    image::Rgb([(c >> 16) as u8, (c >> 8) as u8, c as u8]),
                );
            }
        }
        image
            .save(format!(
                "{root}/assets/textures/palette_cycles/stage_{:02}.png",
                stage + 1
            ))
            .unwrap();
        manifests.push(serde_json::json!({"page_range":bounds[stage],"swatches":format!("textures/palette_cycles/stage_{:02}.png",stage+1)}));
        traces.push(serde_json::json!({"stage":stage,"frames":frames}));
    }
    std::fs::write(
        format!("{root}/assets/textures/palette_cycles/animation.json"),
        serde_json::to_string_pretty(&manifests).unwrap() + "\n",
    )
    .unwrap();
    std::fs::write(
        format!("{root}/tests/fixtures/palette_cycles.json"),
        serde_json::to_string(&traces).unwrap() + "\n",
    )
    .unwrap();
}
