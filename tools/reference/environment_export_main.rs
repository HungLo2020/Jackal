mod audio;
mod game;
mod video;
use game::Game;
static SOUNDS: std::sync::Mutex<Vec<u8>> = std::sync::Mutex::new(Vec::new());
pub fn record_sound(c: u8) {
    SOUNDS.lock().unwrap().push(c);
}
fn main() {
    let root = std::env::args().nth(1).unwrap();
    let mut scenarios = Vec::new();
    for (name, kind, handler, variant, page) in [
        ("headquarters_gate",0x49,0x9bc7,0,7),
        ("formation", 0x32, 0xa684, 0, 7),
        ("leader", 0x33, 0xa6ef, 128, 7),
        ("wing", 0x33, 0xa6ef, 64, 7),
        ("leader_killed",0x33,0xa6ef,128,7),
        ("wing_killed",0x33,0xa6ef,64,7),
        ("wing_cascade",0x33,0xa6ef,64,7),
        ("last", 0x33, 0xa6ef, 1, 7),
        ("hazard7", 0x38, 0x943a, 0, 7),
        ("hazard8", 0x38, 0x943a, 0, 8),
        ("splash", 0x0d, 0x9539, 0, 7),
    ] {
        let mut g = Game::new(false).unwrap();
        g.memory.fill(0);
        g.memory[0x27] = 6;
        g.memory[0x720] = kind;
        g.memory[0x690] = variant;
        g.memory[0x710] = 240;
        g.memory[0x560] = 200;
        g.memory[0x43] = page;
        g.memory[0x40] = 128;
        g.memory[0x530] = 1;
        g.memory[0x5b0] = 80;
        g.memory[0x570] = 160;
        g.memory[0xd4] = 255;
        let mut frames = Vec::new();
        for tick in 0..500u32 {
            if g.memory[0x720] == 0 {
                break;
            }
            g.memory[0x1a] = tick as u8;
            if tick==50 && name.ends_with("_killed") {g.memory[0x520]=2;}
            if tick==50 && name=="wing_cascade" {g.memory[0xd4]=191;}
            SOUNDS.lock().unwrap().clear();
            g.actor_call(handler, 0).unwrap();
            let mut children = Vec::new();
            for i in (1..16).rev() {
                if g.memory[0x720 + i] != 0 {
                    children.push(serde_json::json!({"kind":g.memory[0x720+i],"variant":g.memory[0x690+i],"palette":g.memory[0x540+i],"direction":g.memory[0x6c0+i],"x":g.memory[0x6f0+i]as i8 as f32*256.+g.memory[0x710+i]as f32+g.memory[0x5c0+i]as f32/256.,"y":g.memory[0x700+i]as i8 as f32*256.+g.memory[0x560+i]as f32+g.memory[0x580+i]as f32/256.}));
                    g.memory[0x720 + i] = 0;
                }
            }
            frames.push(serde_json::json!({"tick":tick,"sprite":g.memory[0x500],"palette":g.memory[0x540],"alive":g.memory[0x720]!=0,"flags":g.memory[0x730],"state":g.memory[0x520]&127,"x":g.memory[0x6f0]as i8 as f32*256.+g.memory[0x710]as f32+g.memory[0x5c0]as f32/256.,"y":g.memory[0x700]as i8 as f32*256.+g.memory[0x560]as f32+g.memory[0x580]as f32/256.,"mask":g.memory[0xd4],"children":children,"sounds":SOUNDS.lock().unwrap().clone()}));
        }
        scenarios.push(serde_json::json!({"name":name,"kind":kind,"variant":variant,"page":page,"frames":frames}));
    }
    std::fs::write(
        format!("{root}/tests/fixtures/environment.json"),
        serde_json::to_string_pretty(&scenarios).unwrap() + "\n",
    )
    .unwrap();
}
