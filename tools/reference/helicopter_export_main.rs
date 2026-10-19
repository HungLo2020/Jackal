mod audio;
mod game;
mod video;
use game::Game;
static SOUNDS: std::sync::Mutex<Vec<u8>> = std::sync::Mutex::new(Vec::new());
pub fn record_sound(command: u8) {
    SOUNDS.lock().unwrap().push(command);
}
fn main() {
    let root = std::env::args().nth(1).unwrap();
    let mut scenarios = Vec::new();
    for (name, kind, scroll) in [
        ("left", 0x3d, false),
        ("right_with_passengers", 0x3e, false),
        ("scroll", 0x3d, true),
        ("flyover", 0x3c, false),
    ] {
        let mut g = Game::new(false).unwrap();
        g.memory.fill(0);
        g.memory[0x27] = 6;
        g.memory[0x720] = kind | 0x80;
        g.memory[0x710] = 100;
        g.memory[0x560] = 100;
        let mut frames = Vec::new();
        for tick in 0..1100u32 {
            if g.memory[0x720] == 0 {
                break;
            }
            g.memory[0x1a] = tick as u8;
            g.memory[0x54] = if name == "right_with_passengers" && tick < 200 {
                5
            } else {
                0
            };
            if scroll {
                g.memory[0x44] = (tick % 240) as u8;
                g.memory[0x45] = (tick.saturating_sub(1) % 240) as u8;
            }
            SOUNDS.lock().unwrap().clear();
            g.actor_call(if kind == 0x3c { 0xbb1e } else { 0xbb71 }, 0)
                .unwrap();
            frames.push(serde_json::json!({"tick":tick,"state":g.memory[0x520],"sprite":g.memory[0x500],"x":g.memory[0x6f0]as f32*256.+g.memory[0x710]as f32+g.memory[0x5c0]as f32/256.,"y":(g.memory[0x700]as i8)as f32*256.+g.memory[0x560]as f32+g.memory[0x580]as f32/256.,"alive":g.memory[0x720]!=0,"dock":g.memory[0xd6]!=0,"flip":g.memory[0x6a]&128!=0,"sounds":SOUNDS.lock().unwrap().clone()}));
        }
        scenarios
            .push(serde_json::json!({"name":name,"kind":kind,"scroll":scroll,"frames":frames}));
    }
    std::fs::write(
        format!("{root}/tests/fixtures/rescue_helicopter.json"),
        serde_json::to_string_pretty(&scenarios).unwrap() + "\n",
    )
    .unwrap();
}
