mod audio;
mod game;
mod video;
use std::{collections::HashMap, fs, path::Path};
fn wav(path: &Path, samples: &[i16]) {
    let n = samples.len() as u32 * 2;
    let mut out = Vec::new();
    out.extend(b"RIFF");
    out.extend((36 + n).to_le_bytes());
    out.extend(b"WAVEfmt ");
    out.extend(16u32.to_le_bytes());
    out.extend(1u16.to_le_bytes());
    out.extend(1u16.to_le_bytes());
    out.extend(44100u32.to_le_bytes());
    out.extend(88200u32.to_le_bytes());
    out.extend(2u16.to_le_bytes());
    out.extend(16u16.to_le_bytes());
    out.extend(b"data");
    out.extend(n.to_le_bytes());
    for s in samples {
        out.extend(s.to_le_bytes());
    }
    fs::write(path, out).unwrap();
}
fn main() {
    let destination = std::env::args().nth(1).expect("output assets directory");
    let output = Path::new(&destination);
    let audio_dir = output.join("audio");
    let dest = audio_dir.as_path();
    fs::create_dir_all(dest).unwrap();
    let ids = [
        1, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 14, 16, 17, 18, 19, 21, 22, 23, 25, 27, 29, 31, 33, 35,
        37, 38, 39, 40, 42, 45, 46, 48, 52, 56, 60, 64, 68, 72, 76, 80, 84, 88,
    ];
    let mut entries = Vec::new();
    for id in ids {
        let mut g = game::Game::new(false).unwrap();
        g.sound_call(0x8687, id).unwrap();
        let mut states = HashMap::new();
        let mut samples = Vec::new();
        let mut loop_start = None;
        let mut frames = 0;
        for frame in 0..36000 {
            g.sound_call(0x8008, 0).unwrap();
            samples.extend(g.audio.render());
            frames = frame + 1;
            let mut state = g.memory[0x120..0x1c0].to_vec();
            state.push(g.memory[0xee]);
            if frame > 30 {
                if let Some(previous) = states.insert(state, samples.len()) {
                    loop_start = Some(previous);
                    break;
                }
            }
            if g.memory[0x127..0x12e].iter().all(|&v| v == 0) {
                for _ in 0..15 {
                    samples.extend(g.audio.render());
                }
                break;
            }
        }
        assert!(
            frames < 36000,
            "sound {id:02x} did not finish or loop within ten minutes"
        );
        wav(&dest.join(format!("sound_{id:02x}.wav")), &samples);
        println!(
            "{id:02x}: {} samples {frames} frames loop {loop_start:?}",
            samples.len()
        );
        entries.push(format!("{{\"command\":{id},\"file\":\"audio/sound_{id:02x}.wav\",\"samples\":{},\"loop_start_sample\":{}}}",samples.len(),loop_start.map_or("null".into(),|v|v.to_string())));
    }
    fs::write(dest.parent().unwrap().join("audio.json"),format!("{{\"sample_rate\":44100,\"channels\":1,\"format\":\"PCM16\",\"provenance\":\"Original Jackal sound sequence commands rendered offline from recovered driver; all top-level sound-table entries\",\"sounds\":[{}]}}\n",entries.join(",\n"))).unwrap();
}
