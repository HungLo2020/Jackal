//! Native audio synthesis from the game's APU register writes; no CPU execution.
//! Includes Jackal's two DMC percussion samples. Register writes are frame-timed;
//! cycle-exact APU frame IRQs and CPU DMA stalls are not modeled.
use rodio::{buffer::SamplesBuffer, OutputStream, Sink};

const RATE: u32 = 44_100;
const CPU: f64 = 1_789_773.0;
const FRAME_RATE: f64 = 60.0988;
const DMC_PERIOD: [u16; 16] = [
    428, 380, 340, 320, 286, 254, 226, 214, 190, 160, 142, 128, 106, 85, 72, 54,
];

#[derive(Default)]
struct Dmc {
    rom: Vec<u8>,
    control: u8,
    start: u8,
    length: u8,
    address: u16,
    remaining: u16,
    shift: u8,
    bits: u8,
    clock: f64,
    irq: bool,
}
impl Dmc {
    fn restart(&mut self) {
        self.address = 0xc000 | ((self.start as u16) << 6);
        self.remaining = (self.length as u16) * 16 + 1;
    }
    fn sample(&mut self, dac: &mut u8) {
        self.clock += CPU / (RATE as f64 * DMC_PERIOD[(self.control & 15) as usize] as f64);
        while self.clock >= 1.0 {
            self.clock -= 1.0;
            if self.bits == 0 && self.remaining > 0 {
                // Both recovered samples are wholly inside the fixed $c000 bank.
                self.shift = self
                    .rom
                    .get(self.address.wrapping_sub(0xc000) as usize)
                    .copied()
                    .unwrap_or(0);
                self.address = if self.address == 0xffff {
                    0x8000
                } else {
                    self.address + 1
                };
                self.bits = 8;
                self.remaining -= 1;
                if self.remaining == 0 {
                    if self.control & 0x40 != 0 {
                        self.restart();
                    } else if self.control & 0x80 != 0 {
                        self.irq = true;
                    }
                }
            }
            if self.bits > 0 {
                if self.shift & 1 != 0 {
                    if *dac <= 125 {
                        *dac += 2;
                    }
                } else if *dac >= 2 {
                    *dac -= 2;
                }
                self.shift >>= 1;
                self.bits -= 1;
            }
        }
    }
}
const LENGTH: [u8; 32] = [
    10, 254, 20, 2, 40, 4, 80, 6, 160, 8, 60, 10, 14, 12, 26, 14, 12, 16, 24, 18, 48, 20, 96, 22,
    192, 24, 72, 26, 16, 28, 32, 30,
];
const NOISE_PERIOD: [u16; 16] = [
    4, 8, 16, 32, 64, 96, 128, 160, 202, 254, 380, 508, 762, 1016, 2034, 4068,
];

#[derive(Default, Clone, Copy)]
struct Envelope {
    start: bool,
    divider: u8,
    decay: u8,
}
impl Envelope {
    fn clock(&mut self, control: u8) {
        if self.start {
            self.start = false;
            self.decay = 15;
            self.divider = control & 15;
        } else if self.divider != 0 {
            self.divider -= 1;
        } else {
            self.divider = control & 15;
            if self.decay != 0 {
                self.decay -= 1;
            } else if control & 0x20 != 0 {
                self.decay = 15;
            }
        }
    }
    fn volume(&self, control: u8) -> f64 {
        if control & 0x10 != 0 {
            (control & 15) as f64
        } else {
            self.decay as f64
        }
    }
}

#[derive(Default)]
struct Pulse {
    control: u8,
    sweep: u8,
    timer: u16,
    length: u8,
    phase: f64,
    envelope: Envelope,
    sweep_divider: u8,
    sweep_reload: bool,
}
impl Pulse {
    fn target(&self, first: bool) -> i32 {
        let delta = (self.timer >> (self.sweep & 7)) as i32;
        if self.sweep & 8 != 0 {
            self.timer as i32 - delta - i32::from(first)
        } else {
            self.timer as i32 + delta
        }
    }
    fn half(&mut self, first: bool) {
        if self.control & 0x20 == 0 && self.length > 0 {
            self.length -= 1;
        }
        let target = self.target(first);
        if self.sweep_divider == 0
            && self.sweep & 0x80 != 0
            && self.sweep & 7 != 0
            && self.timer >= 8
            && (0..=0x7ff).contains(&target)
        {
            self.timer = target as u16;
        }
        if self.sweep_divider == 0 || self.sweep_reload {
            self.sweep_divider = (self.sweep >> 4) & 7;
            self.sweep_reload = false;
        } else {
            self.sweep_divider -= 1;
        }
    }
    fn sample(&mut self, first: bool) -> f64 {
        self.phase = (self.phase + CPU / (16.0 * (self.timer as f64 + 1.0) * RATE as f64)).fract();
        if self.length == 0 || self.timer < 8 || self.target(first) > 0x7ff {
            return 0.0;
        }
        let duty = [0.125, 0.25, 0.5, 0.75][(self.control >> 6) as usize];
        if self.phase < duty {
            self.envelope.volume(self.control)
        } else {
            0.0
        }
    }
}

struct Synth {
    pulse: [Pulse; 2],
    enabled: u8,
    triangle_control: u8,
    triangle_timer: u16,
    triangle_length: u8,
    triangle_linear: u8,
    triangle_reload: bool,
    triangle_phase: f64,
    noise_control: u8,
    noise_period: u8,
    noise_length: u8,
    noise_envelope: Envelope,
    noise_shift: u16,
    noise_clock: f64,
    dac: u8,
    dmc: Dmc,
    sequence_clock: f64,
    sequence_step: u8,
    five_step: bool,
    dc_previous: f64,
    dc_output: f64,
}
impl Default for Synth {
    fn default() -> Self {
        Self {
            pulse: [Pulse::default(), Pulse::default()],
            enabled: 0,
            triangle_control: 0,
            triangle_timer: 0,
            triangle_length: 0,
            triangle_linear: 0,
            triangle_reload: false,
            triangle_phase: 0.0,
            noise_control: 0,
            noise_period: 0,
            noise_length: 0,
            noise_envelope: Envelope::default(),
            noise_shift: 1,
            noise_clock: 0.0,
            dac: 0,
            dmc: Dmc::default(),
            sequence_clock: 0.0,
            sequence_step: 0,
            five_step: false,
            dc_previous: 0.0,
            dc_output: 0.0,
        }
    }
}
impl Synth {
    fn write(&mut self, address: u16, value: u8) {
        match address {
            0x4000..=0x4007 => {
                let channel = ((address - 0x4000) / 4) as usize;
                let p = &mut self.pulse[channel];
                match address & 3 {
                    0 => p.control = value,
                    1 => {
                        p.sweep = value;
                        p.sweep_reload = true;
                    }
                    2 => p.timer = (p.timer & 0x700) | value as u16,
                    _ => {
                        p.timer = (p.timer & 0xff) | (((value & 7) as u16) << 8);
                        if self.enabled & (1 << channel) != 0 {
                            p.length = LENGTH[(value >> 3) as usize];
                        }
                        p.phase = 0.0;
                        p.envelope.start = true;
                    }
                }
            }
            0x4008 => self.triangle_control = value,
            0x400a => self.triangle_timer = (self.triangle_timer & 0x700) | value as u16,
            0x400b => {
                self.triangle_timer = (self.triangle_timer & 0xff) | (((value & 7) as u16) << 8);
                if self.enabled & 4 != 0 {
                    self.triangle_length = LENGTH[(value >> 3) as usize];
                }
                self.triangle_reload = true;
            }
            0x400c => self.noise_control = value,
            0x400e => self.noise_period = value,
            0x400f => {
                if self.enabled & 8 != 0 {
                    self.noise_length = LENGTH[(value >> 3) as usize];
                }
                self.noise_envelope.start = true;
            }
            0x4010 => {
                self.dmc.control = value;
                if value & 0x80 == 0 {
                    self.dmc.irq = false;
                }
            }
            0x4011 => self.dac = value & 0x7f,
            0x4012 => self.dmc.start = value,
            0x4013 => self.dmc.length = value,
            0x4015 => {
                self.enabled = value;
                self.dmc.irq = false;
                if value & 0x10 == 0 {
                    self.dmc.remaining = 0;
                } else if self.dmc.remaining == 0 {
                    self.dmc.restart();
                }
                for i in 0..2 {
                    if value & (1 << i) == 0 {
                        self.pulse[i].length = 0;
                    }
                }
                if value & 4 == 0 {
                    self.triangle_length = 0;
                }
                if value & 8 == 0 {
                    self.noise_length = 0;
                }
            }
            0x4017 => {
                self.five_step = value & 0x80 != 0;
                self.sequence_step = 0;
                self.sequence_clock = 0.0;
                if self.five_step {
                    self.quarter();
                    self.half();
                }
            }
            _ => {}
        }
    }
    fn quarter(&mut self) {
        for p in &mut self.pulse {
            p.envelope.clock(p.control);
        }
        self.noise_envelope.clock(self.noise_control);
        if self.triangle_reload {
            self.triangle_linear = self.triangle_control & 0x7f;
        } else if self.triangle_linear > 0 {
            self.triangle_linear -= 1;
        }
        if self.triangle_control & 0x80 == 0 {
            self.triangle_reload = false;
        }
    }
    fn half(&mut self) {
        self.pulse[0].half(true);
        self.pulse[1].half(false);
        if self.triangle_control & 0x80 == 0 && self.triangle_length > 0 {
            self.triangle_length -= 1;
        }
        if self.noise_control & 0x20 == 0 && self.noise_length > 0 {
            self.noise_length -= 1;
        }
    }
    fn sample(&mut self) -> f32 {
        self.dmc.sample(&mut self.dac);
        self.sequence_clock += CPU / (7457.0 * RATE as f64);
        if self.sequence_clock >= 1.0 {
            self.sequence_clock -= 1.0;
            let step = self.sequence_step;
            if !self.five_step || step != 3 {
                self.quarter();
            }
            if step == 1 || (!self.five_step && step == 3) || (self.five_step && step == 4) {
                self.half();
            }
            self.sequence_step = (step + 1) % if self.five_step { 5 } else { 4 };
        }
        let pulses = self.pulse[0].sample(true) + self.pulse[1].sample(false);
        let triangle =
            if self.triangle_length > 0 && self.triangle_linear > 0 && self.triangle_timer > 1 {
                self.triangle_phase = (self.triangle_phase
                    + CPU / (32.0 * (self.triangle_timer as f64 + 1.0) * RATE as f64))
                    .fract();
                let step = (self.triangle_phase * 32.0) as u8;
                if step < 16 {
                    (15 - step) as f64
                } else {
                    (step - 16) as f64
                }
            } else {
                0.0
            };
        self.noise_clock +=
            CPU / (RATE as f64 * NOISE_PERIOD[(self.noise_period & 15) as usize] as f64);
        while self.noise_clock >= 1.0 {
            self.noise_clock -= 1.0;
            let tap = if self.noise_period & 0x80 != 0 { 6 } else { 1 };
            let feedback = (self.noise_shift ^ (self.noise_shift >> tap)) & 1;
            self.noise_shift = (self.noise_shift >> 1) | (feedback << 14);
        }
        let noise = if self.noise_length != 0 && self.noise_shift & 1 == 0 {
            self.noise_envelope.volume(self.noise_control)
        } else {
            0.0
        };
        let pulse_mix = if pulses > 0.0 {
            95.88 / (8128.0 / pulses + 100.0)
        } else {
            0.0
        };
        let tnd = triangle / 8227.0 + noise / 12241.0 + self.dac as f64 / 22638.0;
        let mixed = pulse_mix
            + if tnd > 0.0 {
                159.79 / (1.0 / tnd + 100.0)
            } else {
                0.0
            };
        // Remove the DC component of the hardware's unipolar mixer.
        self.dc_output = mixed - self.dc_previous + 0.995 * self.dc_output;
        self.dc_previous = mixed;
        (self.dc_output * 0.8) as f32
    }
}

pub struct Audio {
    synth: Synth,
    output: Option<(OutputStream, Sink)>,
    sample_remainder: f64,
}
impl Audio {
    pub fn new(enabled: bool) -> Self {
        let output = if enabled {
            OutputStream::try_default()
                .ok()
                .and_then(|(stream, handle)| Sink::try_new(&handle).ok().map(|sink| (stream, sink)))
        } else {
            None
        };
        Self {
            synth: Synth::default(),
            output,
            sample_remainder: 0.0,
        }
    }
    /// Set the fixed 16 KiB bank mapped at $c000, containing Jackal's DMC data.
    pub fn set_dmc_rom(&mut self, fixed_bank: &[u8]) {
        self.synth.dmc.rom = fixed_bank.to_vec();
    }
    pub fn write(&mut self, address: u16, value: u8) {
        self.synth.write(address, value);
    }
    pub fn status(&self) -> u8 {
        u8::from(self.synth.pulse[0].length > 0)
            | (u8::from(self.synth.pulse[1].length > 0) << 1)
            | (u8::from(self.synth.triangle_length > 0) << 2)
            | (u8::from(self.synth.noise_length > 0) << 3)
            | (u8::from(self.synth.dmc.remaining > 0) << 4)
            | (u8::from(self.synth.dmc.irq) << 7)
    }
    pub fn frame(&mut self) {
        self.sample_remainder += RATE as f64 / FRAME_RATE;
        let count = self.sample_remainder as usize;
        self.sample_remainder -= count as f64;
        let samples: Vec<f32> = (0..count).map(|_| self.synth.sample()).collect();
        if let Some((_, sink)) = &self.output {
            // A stalled window must not leave seconds of stale sound queued.
            if sink.len() < 5 {
                sink.append(SamplesBuffer::new(1, RATE, samples));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn dmc_decodes_bits_finishes_and_loops() {
        let mut s = Synth::default();
        s.dmc.rom = vec![0xff; 16384];
        s.write(0x4010, 0x8f);
        s.write(0x4011, 64);
        s.write(0x4012, 0);
        s.write(0x4013, 0);
        s.write(0x4015, 0x10);
        for _ in 0..20 {
            s.sample();
        }
        assert_eq!(s.dac, 80);
        assert_eq!(s.dmc.remaining, 0);
        assert!(s.dmc.irq);
        s.write(0x4010, 0x4f);
        s.write(0x4015, 0x10);
        for _ in 0..1000 {
            s.sample();
        }
        assert_eq!(s.dac, 126);
        assert_eq!(s.dmc.remaining, 1);
        assert!(!s.dmc.irq);
        s.write(0x4015, 0);
        assert_eq!(s.dmc.remaining, 0);
    }
    #[test]
    fn fractional_frame_samples_track_ntsc_clock() {
        let mut a = Audio::new(false);
        for _ in 0..600 {
            a.frame();
        }
        let expected = (600.0 * RATE as f64 / FRAME_RATE).fract();
        assert!((a.sample_remainder - expected).abs() < 1e-7);
    }
    #[test]
    fn silence_and_pulse() {
        let mut s = Synth::default();
        assert!((0..1000).all(|_| s.sample() == 0.0));
        s.write(0x4015, 1);
        s.write(0x4000, 0x9f);
        s.write(0x4002, 0xfd);
        s.write(0x4003, 0x08);
        let energy: f32 = (0..4410).map(|_| s.sample().abs()).sum();
        assert!(energy > 20.0);
        s.write(0x4015, 0);
        for _ in 0..4410 {
            s.sample();
        }
        assert!(s.sample().abs() < 0.0001);
    }
    #[test]
    fn lengths_expire_and_noise_runs() {
        let mut s = Synth::default();
        s.write(0x4015, 8);
        s.write(0x400c, 0x1f);
        s.write(0x400e, 4);
        s.write(0x400f, 0x18);
        assert_eq!(s.noise_length, 2);
        let energy: f32 = (0..500).map(|_| s.sample().abs()).sum();
        assert!(energy > 0.1);
        for _ in 0..4410 {
            s.sample();
        }
        assert_eq!(s.noise_length, 0);
    }
}
