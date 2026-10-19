mod audio;
mod game;
mod video;
use game::Game;
fn main() {
    let root = std::env::args().nth(1).unwrap();
    std::fs::create_dir_all(&root).unwrap();
    let mut samples = Vec::new();
    for stage in 0..6 {
        let mut g = Game::new(false).unwrap();
        for f in 0..900 {
            g.update([if f == 120 { 8 } else { 0 }, 0]).unwrap();
        }
        g.memory[0x30] = stage;
        g.memory[0x18] = 5;
        g.memory[0x2c] = 1;
        g.memory[0x34] = 1;
        for _ in 0..90 {
            g.update([0; 2]).unwrap();
        }
        let initial_video = g.video.export_snapshot();
        for target in [480u32, 720, 960, 1040, 1200, 1440, 1680, 1920, 2160] {
            for _ in 0..5000 {
                let distance = u32::from(g.memory[0x43]) * 240 + u32::from(g.memory[0x44]);
                if distance >= target {
                    break;
                }
                g.memory[0x52] = 255;
                g.memory[0x53] = 255;
                g.memory[0x3c] = 1;
                g.update([0; 2]).unwrap();
            }
            let distance = u32::from(g.memory[0x43]) * 240 + u32::from(g.memory[0x44]);
            assert_eq!(distance, target);
            g.memory[0x3c] = 0;
            for _ in 0..3 {
                g.update([0; 2]).unwrap();
            }
            assert_eq!(
                u32::from(g.memory[0x43]) * 240 + u32::from(g.memory[0x44]),
                target
            );
            let name = format!("stage_{}_distance_{target}", stage + 1);
            // Geometry evidence fixes each stage's initial pattern/palette phase.
            // Live water and palette animation are tested independently.
            let live_video = g.video.export_snapshot();
            let mut fixed_video = live_video;
            fixed_video[..0x2000].copy_from_slice(&initial_video[..0x2000]);
            fixed_video[0x3f00..0x3f20].copy_from_slice(&initial_video[0x3f00..0x3f20]);
            g.video.restore_export_snapshot(fixed_video);
            let original_oam = g.video.oam;
            g.video.oam.fill(255);
            g.video.screenshot(&format!("{root}/{name}_background.png"));
            let mut actor = None;
            if let Some(slot) = (0..16).find(|&i| {
                matches!(
                    g.memory[0x720 + i] & 127,
                    5 | 6 | 12 | 16 | 18 | 31 | 33 | 34 | 42 | 43 | 44 | 45 | 46 | 47 | 48
                ) && g.memory[0x520 + i] & 128 == 0
                    && g.memory[0x500 + i] != 0
                    && (8..248).contains(&g.memory[0x5a0 + i])
                    && (24..208).contains(&g.memory[0x560 + i])
            }) {
                let memory = g.memory;
                let frame = g.memory[0x500 + slot];
                g.memory[0x500..0x520].fill(0);
                g.memory[0x500 + slot] = frame;
                g.memory[0x18] = 0;
                g.draw_alignment().unwrap();
                g.video.oam.copy_from_slice(&g.memory[0x200..0x300]);
                g.video.screenshot(&format!("{root}/{name}_turret.png"));
                actor = Some(
                    serde_json::json!({"source_type":memory[0x720+slot]&127,"frame":frame,"x":memory[0x5a0+slot],"y":memory[0x560+slot],"palette":memory[0x540+slot],"flips":memory[0x6a+slot]}),
                );
                g.memory = memory;
            }
            g.video.oam = original_oam;
            g.video.restore_export_snapshot(live_video);
            samples.push(serde_json::json!({"stage":stage,"distance":distance,"camera_x":g.memory[0x40],"name":name,"actor":actor,"fixed_initial_art_phase":true}));
        }
    }
    std::fs::write(
        format!("{root}/samples.json"),
        serde_json::to_string_pretty(&samples).unwrap() + "\n",
    )
    .unwrap();
}
