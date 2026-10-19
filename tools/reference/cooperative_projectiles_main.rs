mod audio;
mod game;
mod video;
use game::Game;
fn main() {
    let mut out = vec![];
    for (typ, handler, terrain, variant) in [
        (52, 0xfb3a, 0, 8),
        (57, 0x8008, 0, 2),
        (54, 0xbd8a, 0, 0),
        (55, 0x8ba3, 0, 1),
        (54, 0xbd8a, 1, 0),
        (52, 0xfb3a, 0, 16),
        (57, 0x8008, 0, 0),
        (57, 0x8008, 0, 1),
        (57, 0x8008, 0, 3),
        (55, 0x8ba3, 0, 0),
    ] {
        let mut g = Game::new(false).unwrap();
        g.memory.fill(0);
        g.memory[0x300..0x500].fill(terrain * 0x55);
        g.memory[0x530] = 1;
        g.memory[0x531]=1;g.memory[0x5b1]=64;g.memory[0x571]=64;
        g.memory[0x40] = 128;
        g.memory[0x5b0] = 128;
        g.memory[0x570] = 240;
        g.memory[0x720] = typ;
        g.memory[0x710] = 0;
        g.memory[0x6f0] = 1;
        g.memory[0x560] = 160;
        g.memory[0x540] = if typ == 54 {
            2
        } else if typ == 55 {
            1
        } else {
            0
        };
        if typ == 52 {
            g.memory[0x6a0] = 142;
            g.memory[0x6e0] = variant;
            g.memory[0x6c0] = 8;
        } else if typ == 57 {
            g.memory[0x6a0] = 112;
            g.memory[0x6b0] = variant;
            g.memory[0x6c0] = 24;
        } else if typ == 55 {
            g.memory[0x6c0] = 8;
            g.memory[0x690] = variant;
        } else {
            g.memory[0x6a0] = 16;
            g.memory[0x690] = 17;
            g.memory[0x6b0] = 0x20;
            g.memory[0x6c0] = 8;
            g.memory[0x6d0] = 9;
            g.memory[0x6e0] = 64;
        }
        let mut samples = vec![];
        for tick in 0..500 {
            if tick==80{g.memory[0x531]=0;}
            g.actor_call(handler, 0).unwrap();
            samples.push(serde_json::json!({"tick":tick,"sprite":g.memory[0x500],"palette":g.memory[0x540],"flips":g.memory[0x6a],"x":i16::from_le_bytes([g.memory[0x710],g.memory[0x6f0]]) as f32+g.memory[0x5c0] as f32/256.,"y":i16::from_le_bytes([g.memory[0x560],g.memory[0x700]]) as f32+g.memory[0x580] as f32/256.,"state":g.memory[0x520],"type":g.memory[0x720],"direction":g.memory[0x6c0],"turns":g.memory[0x6d0]}));
            if g.memory[0x720] == 0 {
                break;
            }
        }
        out.push(serde_json::json!({"source_type":typ,"terrain":terrain,"variant":variant,"samples":samples}));
    }
    println!("{}", serde_json::to_string_pretty(&out).unwrap());
}
