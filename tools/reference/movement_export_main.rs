mod audio;
mod game;
mod video;
use game::Game;
pub fn record_sound(_: u8) {}
fn main() {
    let root = std::env::args().nth(1).unwrap();
    std::fs::create_dir_all(format!("{root}/tests/fixtures")).unwrap();
    let mut scenarios = Vec::new();
    for (name, two, inputs) in [
        ("down", false, [32, 0]),
        ("up", false, [16, 0]),
        ("right", false, [128, 0]),
        ("coop_up", true, [16, 0]),
        ("coop_opposite", true, [16, 32]),
        ("coop_right", true, [128, 0]),
        ("coop_separate", true, [64, 128]),
        ("opposite_buttons", false, [16 | 32 | 64 | 128, 0]),
        ("backtrack", false, [16, 0]),
        ("forced_scroll", false, [16, 0]),
    ] {
        let mut g = Game::new(false).unwrap();
        for f in 0..900 {
            g.update([
                if f == 120 {
                    8
                } else if two && f == 100 {
                    4
                } else {
                    0
                },
                0,
            ])
            .unwrap();
        }
        g.memory[0x18] = 5;
        g.memory[0x2c] = 1;
        g.memory[0x34] = 1;
        for _ in 0..90 {
            g.update([0, 0]).unwrap();
        }
        let mut frames = Vec::new();
        for f in 0..320 {
            g.memory[0x52] = 255;
            g.memory[0x53] = 255;
            let input = if name == "backtrack" && f >= 128 {
                [32, 0]
            } else {
                inputs
            };
            if name == "forced_scroll" {
                g.memory[0x3c] = 1;
            }
            g.update(input).unwrap();
            frames.push(serde_json::json!({"tick":f,"inputs":input,"x":[g.memory[0x56],g.memory[0x57]],"y":[g.memory[0x58],g.memory[0x59]],"camera_x":g.memory[0x40],"distance":g.memory[0x43]as u32*240+g.memory[0x44]as u32,"backtrack":g.memory[0x3d]}));
        }
        scenarios.push(serde_json::json!({"name":name,"two_players":two,"forced_scroll":name=="forced_scroll","frames":frames}));
    }
    std::fs::write(
        format!("{root}/tests/fixtures/camera_motion.json"),
        serde_json::to_string(&scenarios).unwrap(),
    )
    .unwrap();
}
