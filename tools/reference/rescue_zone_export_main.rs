mod audio;
mod game;
mod video;
use game::Game;
pub fn record_sound(_: u8) {}
fn main() {
    let root = std::env::args().nth(1).unwrap();
    let mut scenarios = Vec::new();
    for (stage, close) in [
        (0, None),
        (1, None),
        (2, None),
        (3, None),
        (4, None),
        (5, None),
        (5, Some(530)),
    ] {
        let mut g = Game::new(false).unwrap();
        g.memory.fill(0);
        g.memory[0x27] = 6;
        g.memory[0x720] = 63;
        g.memory[0x710] = 240;
        g.memory[0x560] = 200;
        g.memory[0x30] = stage;
        g.memory[0xd6] = 1;
        let mut spawns = Vec::new();
        let mut end = 3200;
        for tick in 0..3200 {
            if Some(tick) == close {
                g.memory[0xd6] = 0;
            }
            g.actor_call(0xbd06, 0).unwrap();
            for i in (1..16).rev() {
                if g.memory[0x720 + i] != 0 {
                    spawns.push(serde_json::json!({"tick":tick,"source_type":g.memory[0x720+i],"x":g.memory[0x6f0+i]as i8 as i32*256+g.memory[0x710+i]as i32,"y":g.memory[0x700+i]as i8 as i32*256+g.memory[0x560+i]as i32,"palette":g.memory[0x540+i]}));
                    g.memory[0x720 + i] = 0;
                }
            }
            if g.memory[0x720] == 0 {
                end = tick;
                break;
            }
        }
        scenarios.push(serde_json::json!({"stage":stage,"close":close,"end":end,"spawns":spawns}));
    }
    std::fs::write(
        format!("{root}/tests/fixtures/rescue_zone.json"),
        serde_json::to_string_pretty(&scenarios).unwrap() + "\n",
    )
    .unwrap();
}
