mod audio;
mod game;
mod video;
use game::Game;
use std::collections::HashMap;
struct Export {
    root: String,
    scratch: String,
    images: HashMap<Vec<u8>, String>,
}
impl Export {
    fn frame(&mut self, g: &mut Game) -> String {
        g.video.export(&self.scratch, 3);
        let data = std::fs::read(format!("{}/stage_03_tiles.png", self.scratch)).unwrap();
        if let Some(path) = self.images.get(&data) {
            return path.clone();
        }
        let path = format!(
            "textures/stage_three_water/water_{:02}.png",
            self.images.len()
        );
        std::fs::write(format!("{}/assets/{path}", self.root), &data).unwrap();
        self.images.insert(data, path.clone());
        path
    }
}
fn main() {
    let root = std::env::args().nth(1).unwrap();
    let mut g = Game::new(false).unwrap();
    for f in 0..900 {
        g.update([if f == 120 { 8 } else { 0 }, 0]).unwrap();
    }
    g.memory[0x30] = 2;
    g.memory[0x18] = 5;
    g.memory[0x2c] = 1;
    g.memory[0x34] = 1;
    let dir = format!("{root}/assets/textures/stage_three_water");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::create_dir_all(format!("{root}/tests/fixtures")).unwrap();
    let scratch = format!("{root}/scratch");
    std::fs::create_dir_all(&scratch).unwrap();
    let mut export = Export {
        root: root.clone(),
        scratch: scratch.clone(),
        images: HashMap::new(),
    };
    let mut samples = vec![];
    let mut images = vec![];
    let mut active_tick = 0;
    for tick in 0..500 {
        let mode = g.memory[0x2c];
        let before = [
            g.memory[0x110],
            g.memory[0x111],
            g.memory[0xba],
            g.memory[0xbb],
        ];
        g.memory[0x52] = 255;
        g.update([0; 2]).unwrap();
        if mode < 2 {
            continue;
        }
        let path = export.frame(&mut g);
        images.push(path.clone());
        samples.push(serde_json::json!({"tick":active_tick,"stage_tick":tick,"mode":mode,"before":before,"after":[g.memory[0x110],g.memory[0x111],g.memory[0xba],g.memory[0xbb]],"tiles":path}));
        active_tick += 1;
    }
    let early = vec![images[263..269].to_vec(), images[285..291].to_vec()];
    for _ in 0..5000 {
        if g.memory[0x43] as u32 * 240 + g.memory[0x44] as u32 >= 2740 {
            break;
        }
        g.memory[0x52] = 255;
        g.memory[0x3c] = 1;
        g.update([0; 2]).unwrap();
    }
    g.memory[0x3c] = 0;
    let mut transition = vec![];
    for tick in 0..48 {
        let before = [
            g.memory[0x110],
            g.memory[0x111],
            g.memory[0xba],
            g.memory[0xbb],
        ];
        g.memory[0x52] = 255;
        g.update([0; 2]).unwrap();
        transition.push(serde_json::json!({"tick":tick,"before":before,"after":[g.memory[0x110],g.memory[0x111],g.memory[0xba],g.memory[0xbb]]}));
    }
    let mut late = vec![];
    for command in [4, 3, 4] {
        g.memory[0xba] = command;
        g.memory[0xbb] = 0;
        let mut phases = vec![];
        for tick in 0..8 {
            g.memory[0x110] = 255;
            g.memory[0x52] = 255;
            g.update([0; 2]).unwrap();
            let path = export.frame(&mut g);
            if tick >= 2 {
                phases.push(path);
            }
        }
        if late.len() > 0 || command == 3 {
            late.push(phases);
        }
    }
    std::fs::write(
        format!("{dir}/animation.json"),
        serde_json::to_string_pretty(
            &serde_json::json!({"initial":images[0],"early":early,"late":late}),
        )
        .unwrap()
            + "\n",
    )
    .unwrap();
    std::fs::write(
        format!("{root}/tests/fixtures/stage_three_water.json"),
        serde_json::to_string(
            &serde_json::json!({"frames":samples,"palette_transition":transition}),
        )
        .unwrap()
            + "\n",
    )
    .unwrap();
    std::fs::remove_dir_all(scratch).unwrap();
}
