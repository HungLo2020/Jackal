//! Original alternating river graphics, stored as native PNG upload phases.
use crate::{hq_effects::ArtSequence, renderer::Texture};
use std::path::Path;
pub struct RiverArt {
    early: [ArtSequence; 3],
    late: [ArtSequence; 3],
}
impl RiverArt {
    pub fn load(root: &Path) -> Result<Self, String> {
        let load = |period: &str| -> Result<[ArtSequence; 3], String> {
            Ok([
                ArtSequence::load(root, &format!("river_{period}_initial"))?,
                ArtSequence::load(root, &format!("river_{period}_0_1"))?,
                ArtSequence::load(root, &format!("river_{period}_1_0"))?,
            ])
        };
        Ok(Self {
            early: load("early")?,
            late: load("late")?,
        })
    }
    pub fn requested_phase(age: u64) -> u8 {
        if age < 16 {
            0
        } else {
            if ((age - 16) / 8) & 1 == 0 {
                1
            } else {
                0
            }
        }
    }
    pub fn textures_with_previous(
        &self,
        after_palette_prepare: bool,
        age: u64,
        previous_phase: Option<u8>,
    ) -> (&Texture, &Texture) {
        if age < 3 {
            if let Some(previous) = previous_phase {
                let phases = if after_palette_prepare {
                    &self.late
                } else {
                    &self.early
                };
                return phases[if previous == 0 { 2 } else { 1 }].textures(u16::MAX);
            }
        }
        self.textures(after_palette_prepare, age)
    }
    pub fn textures(&self, after_palette_prepare: bool, age: u64) -> (&Texture, &Texture) {
        let phases = if after_palette_prepare {
            &self.late
        } else {
            &self.early
        };
        if age < 16 {
            return phases[0].textures(age.saturating_sub(1) as u16);
        }
        let group = (age - 16) / 8;
        let elapsed = ((age - 16) % 8) as u16;
        let current = if group & 1 == 0 { 1 } else { 2 };
        if elapsed == 0 {
            let previous = if group == 0 { 0 } else { 3 - current };
            return phases[previous].textures(u16::MAX);
        }
        phases[current].textures(elapsed - 1)
    }
}
