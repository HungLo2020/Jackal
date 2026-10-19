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
        g.memory[0x52] = 255;
        g.memory[0x53] = 255;
        g.memory[0x3c] = 1;
        g.update([0, 0]).unwrap();
        if (0..16).any(|i| g.memory[0x720 + i] & 127 == 0x4a) {
            break;
        }
    }
    let slot = (0..16)
        .find(|i| g.memory[0x720 + i] & 127 == 0x4a)
        .expect("HQ center");
    for _ in 0..120 {
        g.memory[0x52] = 255;
        g.memory[0x3c] = 0;
        g.update([0, 0]).unwrap();
    }
    g.memory[0x520 + slot] = 2;
    g.memory[0x740 + slot] = 0;
    if std::env::args().any(|s| s == "--building-only") {
        for _ in 0..306 {
            g.memory[0x52] = 255;
            g.update([0, 0]).unwrap();
        }
    } else {
        capture(&mut g, &root, "hq_collapse", slot, 306);
    }
    let initial_memory = g.memory;
    let initial_video = g.video.export_snapshot();
    g.memory[0x720..0x730].fill(0);
    g.memory[0xba] = 0x0a;
    g.memory[0xbb] = 0;
    capture(&mut g, &root, "hq_building_initial_0", slot, 8);
    g.memory = initial_memory;
    g.video.restore_export_snapshot(initial_video);
    for _ in 0..100 {
        g.memory[0x52] = 255;
        g.update([0, 0]).unwrap();
    }
    let building = (0..16).find(|i| g.memory[0x720 + i] & 127 == 0x4b).unwrap();
    // Capture each requested graphics command with actor animation paused.
    let saved = g.memory;
    let saved_video = g.video.export_snapshot();
    g.memory[0x720..0x730].fill(0);
    g.memory[0xba] = 13;
    g.memory[0xbb] = 0;
    for _ in 0..8 {
        g.update([0, 0]).unwrap();
    }
    for (previous, phase) in [
        (3, 0),
        (0, 1),
        (1, 2),
        (2, 3),
        (3, 2),
        (2, 1),
        (1, 0),
        (0, 3),
    ] {
        g.memory[0xba] = 0x0a + phase;
        g.memory[0xbb] = 0;
        capture(
            &mut g,
            &root,
            &format!("hq_building_{previous}_{phase}"),
            building,
            8,
        );
    }
    if std::env::args().any(|s| s == "--building-only") {
        return;
    }
    g.memory = saved;
    g.video.restore_export_snapshot(saved_video);
    g.memory[0x520 + building] = 4;
    g.memory[0x740 + building] = 0;
    capture(&mut g, &root, "hq_final", building, 240);
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
        g.memory[0x3c] = 0;
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
