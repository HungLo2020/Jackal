mod audio;
mod game;
mod video;
use game::Game;
static REQUESTS: std::sync::Mutex<Vec<u8>> = std::sync::Mutex::new(Vec::new());
pub fn record_palette(a: u8) {
    REQUESTS.lock().unwrap().push(a);
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
    let mut samples = vec![];
    for tick in 0..2400 {
        let mode = g.memory[0x2c];
        let distance = g.memory[0x43] as u32 * 240 + g.memory[0x44] as u32;
        g.memory[0x3c] = u8::from(mode == 3 && distance < 2400);
        g.memory[0x52] = 255;
        REQUESTS.lock().unwrap().clear();
        g.update([0; 2]).unwrap();
        if mode != 3 {
            continue;
        }
        samples.push(serde_json::json!({"tick":tick,"distance":g.memory[0x43]as u32*240+g.memory[0x44]as u32,"water":[g.memory[0x110],g.memory[0x111],g.memory[0xba]],"signature":g.video.water_signature(),"palette":g.video.background_colors(),"requests":REQUESTS.lock().unwrap().clone()}));
    }
    println!("{}", serde_json::to_string(&samples).unwrap());
}
