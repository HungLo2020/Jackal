mod audio;
mod game;
mod video;
use game::Game;
static REQUESTS: std::sync::Mutex<Vec<u8>> = std::sync::Mutex::new(Vec::new());
pub fn record_palette(a: u8) {
    REQUESTS.lock().unwrap().push(a);
}
fn sample(g: &mut Game, tick: u32, input: [u8; 2]) -> serde_json::Value {
    REQUESTS.lock().unwrap().clear();
    let before = [
        g.memory[0x2c],
        g.memory[0xc7],
        g.memory[0xc4],
        g.memory[0x21],
        g.memory[0x110],
        g.memory[0xba],
    ];
    g.update(input).unwrap();
    serde_json::json!({"tick":tick,"before":before,"after":[g.memory[0x2c],g.memory[0xc7],g.memory[0xc4],g.memory[0x21],g.memory[0x110],g.memory[0xba]],"requests":REQUESTS.lock().unwrap().clone(),"palette":g.video.palette_evidence()})
}
fn main() {
    let mut g = Game::new(false).unwrap();
    for f in 0..900 {
        g.update([if f == 120 { 8 } else { 0 }, 0]).unwrap();
    }
    g.memory[0x30] = 2;
    g.memory[0x18] = 5;
    g.memory[0x2c] = 1;
    g.memory[0x34] = 1;
    let mut entry = vec![];
    for t in 0..40 {
        entry.push(sample(&mut g, t, [0; 2]));
    }
    for _ in 0..5000 {
        if g.memory[0x43] >= 8 {
            break;
        }
        g.memory[0x52] = 255;
        g.memory[0x3c] = 1;
        g.update([0; 2]).unwrap();
    }
    g.memory[0x3c] = 0;
    let mut cycles = vec![];
    for t in 0..180 {
        g.memory[0x52] = 255;
        cycles.push(sample(&mut g, t, [0; 2]));
    }
    let mut gates = vec![];
    for c7 in [0, 1, 2, 128] {
        for queue in [0, 32, 33] {
            g.memory.fill(0);
            g.memory[0x30] = 2;
            g.memory[0x43] = 8;
            g.memory[0xc7] = c7;
            g.memory[0x21] = queue;
            g.memory[0xba] = 1;
            g.memory[0xbb] = 1;
            REQUESTS.lock().unwrap().clear();
            g.scheduler_call().unwrap();
            gates.push(serde_json::json!({"timer":c7,"queue":queue,"after_timer":g.memory[0xc7],"after_phase":g.memory[0xc4],"requests":REQUESTS.lock().unwrap().clone()}));
        }
    }
    println!(
        "{}",
        serde_json::to_string_pretty(
            &serde_json::json!({"entry":entry,"cycles":cycles,"gates":gates})
        )
        .unwrap()
    );
}
