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
    for _ in 0..6000 {
        if g.memory[0x43] as u32 * 240 + g.memory[0x44] as u32 >= 3007 {
            break;
        }
        g.memory[0x52] = 255;
        g.memory[0x3c] = 1;
        g.update([0, 0]).unwrap();
    }
    capture(&mut g, &root, "hq_gate", 0, 64);
    let path = format!("{root}/assets/texture_phases.json");
    let mut manifest: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    let gate: serde_json::Value = serde_json::from_slice(
        &std::fs::read(format!("{root}/assets/textures/hq_gate/timeline.json")).unwrap(),
    )
    .unwrap();
    let phases = manifest["stages"][5].as_array_mut().unwrap();
    phases.retain(|phase| phase["command"] != "headquarters_gate");
    for phase in gate["phases"].as_array().unwrap() {
        let mut phase = phase.clone();
        phase["distance"] = 3008.into();
        phase["command"] = "headquarters_gate".into();
        phases.push(phase);
    }
    std::fs::write(path, serde_json::to_string_pretty(&manifest).unwrap()).unwrap();
}
fn capture(g: &mut Game, root: &str, scene: &str, slot: usize, count: u32) {
    let dir = format!("{root}/assets/textures/{scene}");
    std::fs::create_dir_all(&dir).unwrap();
    let scratch = format!("{root}/assets/.hq-export");
    std::fs::create_dir_all(&scratch).unwrap();
    let mut phases = Vec::new();
    let mut frames = Vec::new();
    let mut last = [Vec::new(), Vec::new()];
    let mut paths = [String::new(), String::new()];
    for tick in 0..count {
        SOUNDS.lock().unwrap().clear();
        g.memory[0x52] = 255;
        g.memory[0x3c] = if tick == 0 { 1 } else { 0 };
        g.update([0, 0]).unwrap();
        g.video.export(&scratch, 6);
        let mut changed = false;
        for (j, name) in ["tiles", "sprites"].iter().enumerate() {
            let data = std::fs::read(format!("{scratch}/stage_06_{name}.png")).unwrap();
            if data != last[j] {
                paths[j] = format!("textures/{scene}/frame_{tick:03}_{name}.png");
                std::fs::write(format!("{root}/assets/{}", paths[j]), &data).unwrap();
                last[j] = data;
                changed = true;
            }
        }
        if changed {
            phases
                .push(serde_json::json!({"tick_offset":tick,"tiles":paths[0],"sprites":paths[1]}));
        }
        let explosions:Vec<_>=(0..16).filter(|i|g.memory[0x720+i]&127==5).map(|i|serde_json::json!({"slot":i,"x":g.memory[0x6f0+i]as u16*256+g.memory[0x710+i]as u16,"y":g.memory[0x560+i],"sprite":g.memory[0x500+i],"age":g.memory[0x6d0+i]})).collect();
        frames.push(serde_json::json!({"tick":tick,"global_tick":g.memory[0x1a],"state":g.memory[0x520+slot]&127,"explosions":explosions,"sounds":SOUNDS.lock().unwrap().clone(),"boss_alive":g.memory[0x3b],"command":g.memory[0xba],"upload":g.memory[0xbb],"counter":g.memory[0x113],"palette_index":g.memory[0x117]}));
        if scene == "hq_final" && g.memory[0x3b] == 0 {
            break;
        }
    }
    std::fs::write(
        format!("{dir}/timeline.json"),
        serde_json::to_string_pretty(&serde_json::json!({"phases":phases,"frames":frames}))
            .unwrap()
            + "\n",
    )
    .unwrap();
    std::fs::remove_dir_all(scratch).unwrap();
}
