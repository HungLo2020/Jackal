//! Native artwork and explosion positions for the headquarters collapse.
use crate::{math::Vec2, renderer::Texture};
use serde::Deserialize;
use std::path::Path;
#[derive(Deserialize)]
struct SourcePhase {
    tick_offset: u16,
    tiles: String,
    sprites: String,
}
#[derive(Deserialize)]
struct Cue {
    tick: u16,
    sounds: Vec<u8>,
}
#[derive(Deserialize)]
struct Source {
    phases: Vec<SourcePhase>,
    frames: Vec<Cue>,
}
pub(crate) struct ArtSequence {
    phases: Vec<(u16, Texture, Texture)>,
    cues: Vec<Cue>,
}
impl ArtSequence {
    pub(crate) fn load(root: &Path, scene: &str) -> Result<Self, String> {
        let source: Source = serde_json::from_slice(
            &std::fs::read(root.join(format!("textures/{scene}/timeline.json"))).map_err(|e| {
                format!(
                    "{}: {e}",
                    root.join(format!("textures/{scene}/timeline.json"))
                        .display()
                )
            })?,
        )
        .map_err(|e| e.to_string())?;
        let mut phases = Vec::new();
        for phase in source.phases {
            let load = |name: String, size: (u32, u32)| -> Result<Texture, String> {
                let image = image::open(root.join(&name))
                    .map_err(|e| format!("{}: {e}", root.join(&name).display()))?
                    .to_rgba8();
                if image.dimensions() != size {
                    return Err("Invalid headquarters atlas size".into());
                }
                Texture::from_rgba(size.0 as usize, size.1 as usize, image.into_raw())
            };
            if phases
                .last()
                .is_some_and(|(tick, _, _)| *tick >= phase.tick_offset)
            {
                return Err("Unordered headquarters animation".into());
            }
            phases.push((
                phase.tick_offset,
                load(phase.tiles, (128, 512))?,
                load(phase.sprites, (128, 1024))?,
            ));
        }
        if phases.first().map(|p| p.0) != Some(0) {
            return Err("Missing headquarters initial art".into());
        }
        Ok(Self {
            phases,
            cues: source
                .frames
                .into_iter()
                .map(|mut cue| {
                    cue.sounds.retain(|command| {
                        if scene == "hq_final" {
                            return *command == 0x27;
                        }
                        matches!(
                            (cue.tick, *command),
                            (0, 0x17) | (24, 0x59) | (25, 0x27) | (216, 0x48)
                        )
                    });
                    cue
                })
                .collect(),
        })
    }
    /// Elapsed ticks since the center turret's death began. Final atlas persists.
    pub fn textures(&self, tick: u16) -> (&Texture, &Texture) {
        let (_, tiles, sprites) = self
            .phases
            .iter()
            .rev()
            .find(|p| p.0 <= tick)
            .unwrap_or(&self.phases[0]);
        (tiles, sprites)
    }
    pub fn sounds(&self, tick: u16) -> &[u8] {
        self.cues
            .iter()
            .find(|cue| cue.tick == tick)
            .map_or(&[], |cue| cue.sounds.as_slice())
    }
}
pub struct CollapseArt {
    center: ArtSequence,
    final_collapse: ArtSequence,
    building: Vec<(u8, u8, ArtSequence)>,
    initial_building: ArtSequence,
}
impl CollapseArt {
    pub fn load(root: &Path) -> Result<Self, String> {
        Ok(Self {
            center: ArtSequence::load(root, "hq_collapse")?,
            final_collapse: ArtSequence::load(root, "hq_final")?,
            initial_building: ArtSequence::load(root, "hq_building_initial_0")?,
            building: [
                (3, 0),
                (0, 1),
                (1, 2),
                (2, 3),
                (3, 2),
                (2, 1),
                (1, 0),
                (0, 3),
            ]
            .into_iter()
            .map(|(previous, phase)| {
                Ok((
                    previous,
                    phase,
                    ArtSequence::load(root, &format!("hq_building_{previous}_{phase}"))?,
                ))
            })
            .collect::<Result<_, String>>()?,
        })
    }
    pub fn textures(&self, tick: u16) -> (&Texture, &Texture) {
        self.center.textures(tick)
    }
    pub fn sounds(&self, tick: u16) -> &[u8] {
        self.center.sounds(tick)
    }
    pub fn final_textures(&self, tick: u16) -> (&Texture, &Texture) {
        self.final_collapse.textures(tick)
    }
    pub fn final_sounds(&self, tick: u16) -> &[u8] {
        self.final_collapse.sounds(tick)
    }
    fn transition(&self, previous: Option<u8>, current: u8) -> &ArtSequence {
        previous
            .and_then(|previous| {
                self.building
                    .iter()
                    .find(|(from, to, _)| *from == previous && *to == current)
            })
            .map_or(&self.initial_building, |(_, _, art)| art)
    }
    /// Original queued uploads overlap the following graphics request by one tick.
    pub fn building_textures(
        &self,
        prior: Option<u8>,
        previous: Option<u8>,
        current: u8,
        elapsed: u16,
    ) -> (&Texture, &Texture) {
        if elapsed == 0 {
            return match previous {
                Some(previous) => self.transition(prior, previous).textures(2),
                None => self.center.textures(u16::MAX),
            };
        }
        self.transition(previous, current).textures(elapsed - 1)
    }
}
/// Original FD64 position cycle, consumed only when an explosion slot is free.
#[derive(Default)]
pub struct CollapseExplosions {
    next: usize,
}
impl CollapseExplosions {
    pub fn spawn(
        &mut self,
        elapsed: u16,
        global_tick: u64,
        camera_y: f32,
        free_slots: usize,
    ) -> Vec<Vec2> {
        if !(26..305).contains(&elapsed) || global_tick & 15 != 0 {
            return Vec::new();
        }
        const POSITIONS: [(f32, f32); 13] = [
            (192., 32.),
            (256., 48.),
            (224., 64.),
            (320., 48.),
            (240., 32.),
            (336., 48.),
            (176., 64.),
            (288., 64.),
            (240., 32.),
            (256., 48.),
            (192., 32.),
            (288., 64.),
            (208., 64.),
        ];
        let positions: Vec<_> = (0..free_slots.min(3))
            .map(|_| {
                let (x, y) = POSITIONS[self.next];
                self.next = (self.next + 1) % POSITIONS.len();
                Vec2::new(x, camera_y + y)
            })
            .collect();
        if elapsed >= 281 {
            Vec::new()
        } else {
            positions
        }
    }
}

impl CollapseExplosions {
    /// Final collapse inspects actual actor slots from highest to lowest. Occupied
    /// slots affect the spacing even though they do not consume a position entry.
    pub fn final_spawn(
        &mut self,
        elapsed: u16,
        global_tick: u64,
        camera: Vec2,
        building: Vec2,
        occupied: [bool; 16],
    ) -> Vec<Vec2> {
        if !(1..208).contains(&elapsed) || global_tick & 15 != 0 {
            return Vec::new();
        }
        const POINTS: [(u16, u8); 13] = [
            (192, 32),
            (256, 48),
            (224, 64),
            (320, 48),
            (240, 32),
            (336, 48),
            (176, 64),
            (288, 64),
            (240, 32),
            (256, 48),
            (192, 32),
            (288, 64),
            (208, 64),
        ];
        let mut offset = 0x9du8;
        let mut carry = false;
        let mut result = Vec::new();
        for occupied in occupied.iter().rev() {
            let sum = u16::from(offset) + 8 + u16::from(carry);
            offset = sum as u8;
            carry = offset >= 72;
            if carry {
                offset = 0;
            }
            if *occupied {
                continue;
            }
            let (fallback_x, y) = POINTS[self.next];
            self.next = (self.next + 1) % 13;
            let screen_x = (building.x - camera.x) as u8;
            let sum = u16::from(screen_x) + u16::from(camera.x as u8);
            let base = (if sum > 255 { 256 } else { fallback_x & 256 }) + (sum & 255);
            let x = if global_tick & 16 != 0 {
                let sum = u32::from(base) + u32::from(offset);
                carry = sum > 65535;
                sum as u16
            } else {
                carry = base >= u16::from(offset);
                base.wrapping_sub(u16::from(offset))
            };
            result.push(Vec2::new(f32::from(x), camera.y + f32::from(y)));
            if result.len() == 3 {
                break;
            }
        }
        if elapsed >= 192 {
            Vec::new()
        } else {
            result
        }
    }
}
