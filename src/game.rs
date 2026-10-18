use crate::{audio::Audio, video::Video};

mod translated {
    include!("../generated/native_game.rs");
}
const PRG: &[u8] = include_bytes!("../assets/game-data.bin");

pub struct Game {
    pub memory: [u8; 2048],
    pub video: Video,
    pub audio: Audio,
    pub bank: u8,
    pub a: u8,
    pub x: u8,
    pub y: u8,
    sp: u8,
    c: bool,
    z: bool,
    n: bool,
    v: bool,
    interrupt_mask: bool,
    decimal: bool,
    block: u16,
    pub frame: u64,
    buttons: [u8; 2],
    shift: [u8; 2],
    strobe: bool,
    pub transfers: u64,
    elapsed: u32,
}

#[cfg(test)]
mod tests {
    use super::*;
    fn started(two: bool) -> Game {
        let mut game = Game::new(false).unwrap();
        for frame in 0..900 {
            let buttons = if two && frame == 100 {
                4
            } else if frame == 120 {
                8
            } else {
                0
            };
            game.update([buttons, 0]).unwrap();
        }
        assert_eq!(game.memory[0x18], 5, "new game must enter gameplay");
        game
    }
    #[test]
    fn original_menu_and_both_players_work() {
        let mut game = started(true);
        assert_eq!(game.memory[0x22], 1, "Select must choose two players");
        assert!(game.memory[0x31] > 0 && game.memory[0x32] > 0);
        let before = [game.memory[0x5b0], game.memory[0x5b1]];
        for _ in 0..20 {
            game.update([0x80, 0x40]).unwrap();
        }
        assert_ne!(before[0], game.memory[0x5b0], "P1 moves right");
        assert_ne!(before[1], game.memory[0x5b1], "P2 moves left");
    }
    #[test]
    fn six_original_stages_initialize_and_run() {
        for stage in 0..6 {
            let mut game = started(false);
            game.memory[0x30] = stage;
            game.memory[0x18] = 5;
            game.memory[0x2c] = 1;
            game.memory[0x34] = 1;
            for _ in 0..90 {
                game.update([0; 2]).unwrap();
            }
            assert_eq!(
                game.memory[0x2c], 3,
                "stage {stage} reaches active gameplay"
            );
            assert_eq!(game.memory[0x30], stage);
            let mut rng = 0x12345678u32;
            for _ in 0..1800 {
                rng ^= rng << 13;
                rng ^= rng >> 17;
                rng ^= rng << 5;
                let direction = [0x10, 0x20, 0x40, 0x80][(rng as usize >> 8) & 3];
                game.update([direction | ((rng as u8) & 3), 0]).unwrap();
            }
            let pixels = game.video.render();
            assert!(
                pixels.iter().any(|&p| p != pixels[0]),
                "stage {stage} renders graphics"
            );
        }
    }
    #[test]
    fn all_boss_areas_stream_through_original_event_scripts() {
        for stage in 0..6 {
            let mut game = started(false);
            game.memory[0x30] = stage;
            game.memory[0x18] = 5;
            game.memory[0x2c] = 1;
            game.memory[0x34] = 1;
            for _ in 0..90 {
                game.update([0; 2]).unwrap();
            }
            let mut found = false;
            for _ in 0..5000 {
                game.memory[0x52] = 255;
                game.memory[0x53] = 255;
                game.memory[0x3c] = 1;
                game.update([3, 0]).unwrap();
                if game.memory[0x43] >= if stage == 5 { 13 } else { 11 }
                    && game.memory[0x3a] != 0
                    && game.memory[0x3b] != 0
                {
                    found = true;
                    break;
                }
            }
            assert!(
                found,
                "stage {stage} failed to reach boss; page={}, boss={}, remaining={}",
                game.memory[0x43], game.memory[0x3a], game.memory[0x3b]
            );
            for _ in 0..600 {
                game.memory[0x52] = 255;
                game.memory[0x53] = 255;
                game.update([3, 0]).unwrap();
            }
            // Exercise the original victory path from a coherently streamed
            // boss arena. This simulates victory; it does not claim to beat it.
            game.memory[0x3b] = 0;
            game.memory[0x42] = 0xfe;
            game.update([0; 2]).unwrap();
            assert_eq!(game.memory[0x18], 7, "stage {stage} enters victory");
            if stage < 5 {
                for _ in 0..1800 {
                    game.update([0; 2]).unwrap();
                    if game.memory[0x18] == 5 && game.memory[0x30] == stage + 1 {
                        break;
                    }
                }
                assert_eq!(game.memory[0x30], stage + 1, "stage transition");
                assert_eq!(game.memory[0x18], 5);
            } else {
                let mut ending_states = std::collections::BTreeSet::new();
                for _ in 0..6000 {
                    game.update([0; 2]).unwrap();
                    if game.memory[0x18] == 9 {
                        ending_states.insert(game.memory[0x19]);
                    }
                }
                assert!(
                    ending_states.contains(&10),
                    "ending must reach its final screen: {ending_states:?}"
                );
                for _ in 0..180 {
                    game.update([8, 0]).unwrap();
                }
                assert_eq!(
                    game.memory[0x18], 5,
                    "Start after ending begins the next loop"
                );
                assert_eq!(game.memory[0x30], 0);
            }
        }
    }
    #[test]
    fn game_over_reaches_continue_screen() {
        let mut game = started(false);
        game.memory[0x31] = 0;
        game.memory[0x32] = 0;
        let mut states = std::collections::BTreeSet::new();
        for _ in 0..1500 {
            game.update([0; 2]).unwrap();
            states.insert(game.memory[0x18]);
            if game.memory[0x18] == 8 {
                break;
            }
        }
        assert!(states.contains(&6), "original game-over state must execute");
        assert!(states.contains(&8), "continue screen must execute");
        for _ in 0..120 {
            game.update([0; 2]).unwrap();
        }
        game.update([8, 0]).unwrap();
        for _ in 0..180 {
            game.update([0; 2]).unwrap();
        }
        assert_eq!(game.memory[0x18], 5, "Continue restarts gameplay");
        assert!(game.memory[0x31] > 0);
    }
}
impl Game {
    pub fn new(sound: bool) -> Result<Self, String> {
        let mut s = Self {
            memory: [0; 2048],
            video: Video::new(),
            audio: Audio::new(sound),
            bank: 0,
            a: 0,
            x: 0,
            y: 0,
            sp: 0xfd,
            c: false,
            z: false,
            n: false,
            v: false,
            interrupt_mask: true,
            decimal: false,
            block: 0xc23d,
            frame: 0,
            buttons: [0; 2],
            shift: [0; 2],
            strobe: false,
            transfers: 0,
            elapsed: 0,
        };
        s.audio.set_dmc_rom(&PRG[7 * 16384..8 * 16384]);
        s.run_until_idle()?;
        Ok(s)
    }
    fn run_until_idle(&mut self) -> Result<(), String> {
        for _ in 0..2_000_000 {
            if self.block == 0xc28f {
                return Ok(());
            }
            if self.block == 0xc8e4 {
                // Story's timed mid-screen horizontal split. Retained source
                // timing annotations locate it relative to vblank; no CPU loop.
                let row = ((self.elapsed + 28) * 3).saturating_sub(21 * 341) / 341;
                self.video.capture_split_top((row as usize).min(240));
            }
            let addr = self.block;
            self.block = translated::dispatch(self, addr)?;
            self.transfers += 1;
        }
        Err(format!(
            "Frame failed to finish: bank {} ${:04x}",
            self.bank, self.block
        ))
    }
    pub fn update(&mut self, buttons: [u8; 2]) -> Result<(), String> {
        self.buttons = buttons;
        self.video.begin_frame();
        self.video.set_vblank(true);
        // Recovered reset parks at $c28f. The complete update is the $c292
        // vertical-blank routine, invoked once per native frame, without CPU clocks.
        self.push_word(0xc28f);
        self.push(self.flags());
        self.interrupt_mask = true;
        self.block = 0xc292;
        self.elapsed = 7;
        self.run_until_idle()?;
        self.audio.frame();
        self.frame += 1;
        Ok(())
    }
    fn read(&mut self, addr: u16) -> u8 {
        match addr {
            0..=0x1fff => self.memory[(addr & 0x7ff) as usize],
            0x2000..=0x3fff => {
                // Graphics loaders poll vblank; native uploads complete synchronously.
                if addr & 7 == 2 {
                    self.video.set_vblank(true);
                }
                self.video.read_register(0x2000 | (addr & 7))
            }
            0x4015 => self.audio.status(),
            0x4016..=0x4017 => {
                let p = (addr - 0x4016) as usize;
                if self.strobe {
                    self.buttons[p] & 1
                } else {
                    let v = self.shift[p] & 1;
                    self.shift[p] = (self.shift[p] >> 1) | 128;
                    v
                }
            }
            0x8000..=0xbfff => PRG[self.bank as usize * 16384 + (addr as usize & 0x3fff)],
            0xc000..=0xffff => PRG[7 * 16384 + (addr as usize & 0x3fff)],
            _ => 0,
        }
    }
    fn write(&mut self, addr: u16, value: u8) {
        match addr {
            0..=0x1fff => self.memory[(addr & 0x7ff) as usize] = value,
            0x2000..=0x3fff => self.video.write_register(0x2000 | (addr & 7), value),
            0x4014 => {
                self.elapsed += 513 + (self.elapsed & 1);
                let start = (value as usize * 256) & 0x7ff;
                self.video.dma(&self.memory[start..start + 256]);
            }
            0x4016 => {
                self.strobe = value & 1 != 0;
                if self.strobe {
                    self.shift = self.buttons;
                }
            }
            0x4000..=0x4017 => self.audio.write(addr, value),
            0x8000..=0xffff => self.bank = value & 7,
            _ => {}
        }
    }
    fn pointer(&self, address: u8) -> u16 {
        self.memory[address as usize] as u16
            | ((self.memory[address.wrapping_add(1) as usize] as u16) << 8)
    }
    fn nz(&mut self, value: u8) {
        self.n = value & 128 != 0;
        self.z = value == 0;
    }
    fn add(&mut self, value: u8) {
        let result = self.a as u16 + value as u16 + self.c as u16;
        self.v = (!(self.a ^ value) & (self.a ^ result as u8) & 128) != 0;
        self.c = result > 255;
        self.a = result as u8;
        self.nz(self.a);
    }
    fn push(&mut self, value: u8) {
        self.memory[0x100 + self.sp as usize] = value;
        self.sp = self.sp.wrapping_sub(1);
    }
    fn pop(&mut self) -> u8 {
        self.sp = self.sp.wrapping_add(1);
        self.memory[0x100 + self.sp as usize]
    }
    fn push_word(&mut self, value: u16) {
        self.push((value >> 8) as u8);
        self.push(value as u8);
    }
    fn pop_word(&mut self) -> u16 {
        let lo = self.pop() as u16;
        lo | ((self.pop() as u16) << 8)
    }
    fn flags(&self) -> u8 {
        self.c as u8
            | ((self.z as u8) << 1)
            | ((self.interrupt_mask as u8) << 2)
            | ((self.decimal as u8) << 3)
            | 0x20
            | ((self.v as u8) << 6)
            | ((self.n as u8) << 7)
    }
    fn set_flags(&mut self, v: u8) {
        self.c = v & 1 != 0;
        self.z = v & 2 != 0;
        self.interrupt_mask = v & 4 != 0;
        self.decimal = v & 8 != 0;
        self.v = v & 64 != 0;
        self.n = v & 128 != 0;
    }
}
