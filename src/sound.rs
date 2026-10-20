//! Playback of original music and effects exported as ordinary PCM WAV assets.
use rodio::{Decoder, OutputStream, OutputStreamHandle, Sink, Source};
use std::{io::Cursor, time::Duration};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MusicTrack {
    Title,
    Briefing,
    Stage(u8),
    Boss,
    Victory,
    GameOver,
    Ending,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SoundEffect {
    MachineGun,
    Grenade,
    Rocket,
    Explosion,
    PlayerDeath,
    InfantryDeath,
    Rescue,
    Pickup,
    Delivery,
}

struct Clip {
    bytes: &'static [u8],
    loop_start: Option<u64>,
}
fn music_id(track: MusicTrack) -> u8 {
    match track {
        MusicTrack::Title => 0x30,
        MusicTrack::Briefing => 0x34,
        MusicTrack::Stage(stage) => [0x38, 0x3c, 0x40][stage as usize % 3],
        MusicTrack::Boss => 0x23,
        MusicTrack::Victory => 0x4c,
        MusicTrack::GameOver => 0x50,
        MusicTrack::Ending => 0x54,
    }
}
fn effect_id(effect: SoundEffect) -> u8 {
    match effect {
        SoundEffect::MachineGun => 0x09,
        SoundEffect::Grenade => 0x11,
        SoundEffect::Rocket => 0x13,
        SoundEffect::Explosion => 0x15,
        SoundEffect::PlayerDeath => 0x25,
        SoundEffect::InfantryDeath => 0x12,
        SoundEffect::Rescue => 0x1d,
        SoundEffect::Pickup => 0x21,
        SoundEffect::Delivery => 0x1f,
    }
}

pub struct Sound {
    output: Option<(OutputStream, OutputStreamHandle)>,
    music_sink: Option<Sink>,
    effects: Vec<Sink>,
    current: Option<u8>,
    paused: bool,
}
impl Sound {
    pub fn new(enabled: bool) -> Self {
        Self {
            output: if enabled {
                match OutputStream::try_default() {
                    Ok(output) => Some(output),
                    Err(error) => {
                        eprintln!("Audio unavailable: {error}. Continuing without sound.");
                        None
                    }
                }
            } else {
                None
            },
            music_sink: None,
            effects: Vec::new(),
            current: None,
            paused: false,
        }
    }
    pub fn music(&mut self, track: MusicTrack) {
        self.music_command(music_id(track));
    }
    fn music_command(&mut self, command: u8) {
        if self.current == Some(command) {
            return;
        }
        self.stop_music();
        self.current = Some(command);
        let Some((_, handle)) = &self.output else {
            return;
        };
        let Ok(sink) = Sink::try_new(handle) else {
            return;
        };
        let clip = clip(command);
        let Ok(source) = Decoder::new(Cursor::new(clip.bytes)) else {
            return;
        };
        if let Some(start) = clip.loop_start {
            let duration = Duration::from_secs_f64(start as f64 / 44100.0);
            sink.append(source.take_duration(duration));
            if let Ok(source) = Decoder::new(Cursor::new(clip.bytes)) {
                sink.append(source.skip_duration(duration).repeat_infinite());
            }
        } else {
            sink.append(source);
        }
        sink.set_volume(0.75);
        if self.paused {
            sink.pause();
        }
        self.music_sink = Some(sink);
    }
    pub fn effect(&mut self, effect: SoundEffect) {
        self.effect_command(effect_id(effect));
    }
    /// Scene timelines retain original sound-cue IDs as asset identifiers.
    pub fn command(&mut self, command: u8) {
        match command {
            0 | 0x59 => {
                self.stop_music();
                for sink in self.effects.drain(..) {
                    sink.stop();
                }
            }
            0x23 | 0x30 | 0x34 | 0x38 | 0x3c | 0x40 | 0x44 | 0x48 | 0x4c | 0x50 | 0x54 => {
                self.music_command(command)
            }
            _ => self.effect_command(command),
        }
    }
    fn effect_command(&mut self, command: u8) {
        if self.paused {
            return;
        }
        self.effects.retain(|sink| !sink.empty());
        let Some((_, handle)) = &self.output else {
            return;
        };
        let Ok(sink) = Sink::try_new(handle) else {
            return;
        };
        let Ok(source) = Decoder::new(Cursor::new(clip(command).bytes)) else {
            return;
        };
        if self.effects.len() >= 16 {
            self.effects.remove(0).stop();
        }
        sink.append(source);
        sink.set_volume(0.8);
        self.effects.push(sink);
    }
    pub fn pause(&mut self, paused: bool) {
        self.paused = paused;
        if let Some(sink) = &self.music_sink {
            if paused {
                sink.pause();
            } else {
                sink.play();
            }
        }
        for sink in &self.effects {
            if paused {
                sink.pause();
            } else {
                sink.play();
            }
        }
    }
    pub fn stop_music(&mut self) {
        if let Some(sink) = self.music_sink.take() {
            sink.stop();
        }
        self.current = None;
    }
}

fn clip(id: u8) -> Clip {
    match id {
        1 => Clip {
            bytes: include_bytes!("../assets/audio/effect_01.wav"),
            loop_start: None,
        },
        3 => Clip {
            bytes: include_bytes!("../assets/audio/effect_03.wav"),
            loop_start: None,
        },
        4 => Clip {
            bytes: include_bytes!("../assets/audio/effect_04.wav"),
            loop_start: None,
        },
        5 => Clip {
            bytes: include_bytes!("../assets/audio/effect_05.wav"),
            loop_start: None,
        },
        6 => Clip {
            bytes: include_bytes!("../assets/audio/effect_06.wav"),
            loop_start: None,
        },
        7 => Clip {
            bytes: include_bytes!("../assets/audio/effect_07.wav"),
            loop_start: None,
        },
        8 => Clip {
            bytes: include_bytes!("../assets/audio/effect_08.wav"),
            loop_start: None,
        },
        9 => Clip {
            bytes: include_bytes!("../assets/audio/machine_gun.wav"),
            loop_start: None,
        },
        10 => Clip {
            bytes: include_bytes!("../assets/audio/effect_0a.wav"),
            loop_start: None,
        },
        11 => Clip {
            bytes: include_bytes!("../assets/audio/effect_0b.wav"),
            loop_start: None,
        },
        12 => Clip {
            bytes: include_bytes!("../assets/audio/effect_0c.wav"),
            loop_start: None,
        },
        14 => Clip {
            bytes: include_bytes!("../assets/audio/effect_0e.wav"),
            loop_start: None,
        },
        16 => Clip {
            bytes: include_bytes!("../assets/audio/effect_10.wav"),
            loop_start: None,
        },
        17 => Clip {
            bytes: include_bytes!("../assets/audio/grenade.wav"),
            loop_start: None,
        },
        18 => Clip {
            bytes: include_bytes!("../assets/audio/effect_12.wav"),
            loop_start: None,
        },
        19 => Clip {
            bytes: include_bytes!("../assets/audio/effect_13.wav"),
            loop_start: None,
        },
        21 => Clip {
            bytes: include_bytes!("../assets/audio/explosion.wav"),
            loop_start: None,
        },
        22 => Clip {
            bytes: include_bytes!("../assets/audio/effect_16.wav"),
            loop_start: None,
        },
        23 => Clip {
            bytes: include_bytes!("../assets/audio/effect_17.wav"),
            loop_start: None,
        },
        25 => Clip {
            bytes: include_bytes!("../assets/audio/effect_19.wav"),
            loop_start: None,
        },
        27 => Clip {
            bytes: include_bytes!("../assets/audio/effect_1b.wav"),
            loop_start: None,
        },
        29 => Clip {
            bytes: include_bytes!("../assets/audio/rescue.wav"),
            loop_start: None,
        },
        31 => Clip {
            bytes: include_bytes!("../assets/audio/delivery.wav"),
            loop_start: None,
        },
        33 => Clip {
            bytes: include_bytes!("../assets/audio/pickup.wav"),
            loop_start: None,
        },
        35 => Clip {
            bytes: include_bytes!("../assets/audio/boss.wav"),
            loop_start: Some(490906),
        },
        37 => Clip {
            bytes: include_bytes!("../assets/audio/player_death.wav"),
            loop_start: None,
        },
        38 => Clip {
            bytes: include_bytes!("../assets/audio/effect_26.wav"),
            loop_start: None,
        },
        39 => Clip {
            bytes: include_bytes!("../assets/audio/effect_27.wav"),
            loop_start: None,
        },
        40 => Clip {
            bytes: include_bytes!("../assets/audio/effect_28.wav"),
            loop_start: Some(248021),
        },
        42 => Clip {
            bytes: include_bytes!("../assets/audio/effect_2a.wav"),
            loop_start: None,
        },
        45 => Clip {
            bytes: include_bytes!("../assets/audio/effect_2d.wav"),
            loop_start: None,
        },
        46 => Clip {
            bytes: include_bytes!("../assets/audio/effect_2e.wav"),
            loop_start: None,
        },
        48 => Clip {
            bytes: include_bytes!("../assets/audio/title.wav"),
            loop_start: None,
        },
        52 => Clip {
            bytes: include_bytes!("../assets/audio/briefing.wav"),
            loop_start: Some(564285),
        },
        56 => Clip {
            bytes: include_bytes!("../assets/audio/stage_1.wav"),
            loop_start: Some(245820),
        },
        60 => Clip {
            bytes: include_bytes!("../assets/audio/stage_2.wav"),
            loop_start: Some(785890),
        },
        64 => Clip {
            bytes: include_bytes!("../assets/audio/stage_3.wav"),
            loop_start: Some(23481),
        },
        68 => Clip {
            bytes: include_bytes!("../assets/audio/alternate_stage.wav"),
            loop_start: Some(436606),
        },
        72 => Clip {
            bytes: include_bytes!("../assets/audio/helicopter.wav"),
            loop_start: Some(661146),
        },
        76 => Clip {
            bytes: include_bytes!("../assets/audio/victory.wav"),
            loop_start: None,
        },
        80 => Clip {
            bytes: include_bytes!("../assets/audio/game_over.wav"),
            loop_start: None,
        },
        84 => Clip {
            bytes: include_bytes!("../assets/audio/ending.wav"),
            loop_start: Some(1074271),
        },
        88 => Clip {
            bytes: include_bytes!("../assets/audio/effect_58.wav"),
            loop_start: None,
        },
        _ => unreachable!("unknown sound asset"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn all_original_audio_assets_decode_and_loop_within_bounds() {
        let ids = [
            1, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 14, 16, 17, 18, 19, 21, 22, 23, 25, 27, 29, 31, 33,
            35, 37, 38, 39, 40, 42, 45, 46, 48, 52, 56, 60, 64, 68, 72, 76, 80, 84, 88,
        ];
        for id in ids {
            let asset = clip(id);
            let decoder = Decoder::new(Cursor::new(asset.bytes)).unwrap();
            assert_eq!(decoder.sample_rate(), 44100);
            assert_eq!(decoder.channels(), 1);
            let count = decoder.count() as u64;
            assert!(count > 0);
            if let Some(start) = asset.loop_start {
                assert!(start < count);
            }
        }
    }
    #[test]
    fn muted_playback_handles_all_game_events() {
        let mut sound = Sound::new(false);
        for track in [
            MusicTrack::Title,
            MusicTrack::Briefing,
            MusicTrack::Stage(0),
            MusicTrack::Stage(5),
            MusicTrack::Boss,
            MusicTrack::Victory,
            MusicTrack::GameOver,
            MusicTrack::Ending,
        ] {
            sound.music(track);
        }
        for effect in [
            SoundEffect::MachineGun,
            SoundEffect::Grenade,
            SoundEffect::Rocket,
            SoundEffect::Explosion,
            SoundEffect::PlayerDeath,
            SoundEffect::InfantryDeath,
            SoundEffect::Rescue,
            SoundEffect::Pickup,
            SoundEffect::Delivery,
        ] {
            sound.effect(effect);
        }
        sound.pause(true);
        sound.pause(false);
        sound.stop_music();
        assert_eq!(sound.current, None);
    }
}
