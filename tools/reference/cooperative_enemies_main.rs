mod audio;
mod game;
mod video;
use game::Game;
fn main() {
    let mut out = vec![];
    for (typ, handler, targetx, targety, stage) in [
        (1,0x8263,80,160,0),(2,0x84e6,80,160,0),(3,0x84e6,80,160,0),(4,0xa470,80,160,0),(5,0xb7d8,80,160,0),(6,0xb928,80,160,0),
        (16,0x8cdc,80,0,0),(31,0x8cdc,80,0,0),(47,0xa36d,80,0,0),(48,0xa36d,80,0,0),
        (7, 0x8112, 80, 160, 0),
        (15, 0x8c5f, 80, 160, 0),
        (7, 0x8112, 160, 160, 0),
        (15, 0x8c5f, 160, 160, 5),
        (8, 0x86fb, 80, 160, 0),
        (26, 0x96ea, 80, 160, 0),
        (11, 0x8aaa, 80, 160, 0),
        (14, 0xad4d, 80, 160, 0),
        (12, 0xac22, 80, 160, 0),
        (16, 0x8cdc, 80, 160, 0),
        (31, 0x8cdc, 80, 160, 0),
        (17, 0x921b, 80, 160, 0),
        (30, 0x8d76, 80, 160, 0),
        (18, 0x957c, 80, 160, 0),
        (42, 0xa28d, 80, 160, 0),
        (43, 0xa28d, 80, 160, 0),
        (47, 0xa36d, 160, 100, 0),
        (48, 0xa36d, 0, 100, 0),
        (33, 0x8ee9, 80, 160, 0),
        (34, 0x8ee9, 80, 160, 0),
        (41, 0x92ad, 80, 160, 0),
    ] {
        let mut g = Game::new(false).unwrap();
        g.memory.fill(0);
        g.memory[0x30] = stage;
        g.memory[0x530] = 1;
        let second=if matches!(typ,16|31){[80,144]}else if matches!(typ,47|48){[200,112]}else{[200,144]};
        g.memory[0x531]=1;g.memory[0x5b1]=second[0];g.memory[0x571]=second[1];
        g.memory[0x5b0] = targetx;
        g.memory[0x570] = targety;
        g.memory[0x720] = if matches!(typ, 33 | 34) {
            typ | 128
        } else {
            typ
        };
        if typ == 30 {
            g.memory[0x6c0] = 10;
            g.memory[0x6a] = 64;
        }
        g.memory[0x710] = 80;
        g.memory[0x560] = 80;
        let mut samples = vec![];
        for tick in 0..600 {
            g.memory[0x1a] = tick as u8;
            if tick==300{g.memory[0x531]=0;}
            g.actor_call(handler, 0).unwrap();
            if g.memory[0x720] == 0 {
                break;
            }
            let mut shots = vec![];
            for j in 1..16 {
                if g.memory[0x720 + j] != 0 {
                    shots.push(g.memory[0x720 + j]);
                    g.memory[0x720 + j] = 0;
                }
            }
            samples.push(serde_json::json!({"tick":tick,"sprite":g.memory[0x500],"palette":g.memory[0x540],"direction":g.memory[0x6c0],"target_x":g.memory[0x5b0],"target_y":g.memory[0x570],"flips":g.memory[0x6a],"x":i16::from_le_bytes([g.memory[0x710],g.memory[0x6f0]]) as f32+g.memory[0x5c0] as f32/256.,"y":i16::from_le_bytes([g.memory[0x560],g.memory[0x700]]) as f32+g.memory[0x580] as f32/256.,"state":g.memory[0x520],"timer":g.memory[0x670],"shots":shots}));
        }
        out.push(serde_json::json!({"source_type":typ,"stage":stage,"second_target":second,"target":[targetx,targety],"samples":samples}));
    }
    println!("{}", serde_json::to_string_pretty(&out).unwrap());
}
