//! Session-local destruction overlays. Exported level data remains immutable;
//! collision and drawing read the same changed cells after a building breaks.
use crate::{
    assets::{DestructionDefinition, Level},
    math::Vec2,
};
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TilePatch {
    pub tile: u16,
    pub palette: u8,
    pub collision: u8,
}

#[derive(Clone, Debug, Default)]
pub struct TerrainChanges {
    pub patches: BTreeMap<usize, TilePatch>,
}

impl TerrainChanges {
    pub fn clear(&mut self) {
        self.patches.clear();
    }

    /// Apply the original destroyed building's background rectangle. Background
    /// palette attributes survive destruction, just as they do in the reference.
    pub fn destroy_building(
        &mut self,
        level: &Level,
        center: Vec2,
        definition: &DestructionDefinition,
    ) {
        if !center.x.is_finite() || !center.y.is_finite() || level.tile_size == 0 {
            return;
        }
        let size = level.tile_size as f32;
        let x0 = ((center.x + definition.offset_x as f32) / size).floor() as i32;
        let y0 = ((center.y + definition.offset_y as f32) / size).floor() as i32;
        for y in 0..definition.height {
            for x in 0..definition.width {
                let (xx, yy) = (x0 + x as i32, y0 + y as i32);
                if xx < 0
                    || yy < 0
                    || xx as usize >= level.width_tiles
                    || yy as usize >= level.height_tiles
                {
                    continue;
                }
                let index = yy as usize * level.width_tiles + xx as usize;
                self.patches.insert(
                    index,
                    TilePatch {
                        tile: definition.tiles[y * definition.width + x],
                        palette: level.palettes[index],
                        collision: definition.collision,
                    },
                );
            }
        }
    }

    /// Permanent background change emitted by an encounter actor.
    pub fn apply_boss_patch(
        &mut self,
        level: &Level,
        center: Vec2,
        patch: &crate::world_art::BossTilePatch,
    ) {
        if !center.x.is_finite() || !center.y.is_finite() {
            return;
        }
        let x0 = ((center.x + patch.offset_x as f32) / 8.).floor() as i32;
        let y0 = ((center.y + patch.offset_y as f32) / 8.).floor() as i32;
        for y in 0..patch.height {
            for x in 0..patch.width {
                let (xx, yy) = (x0 + x as i32, y0 + y as i32);
                if xx < 0
                    || yy < 0
                    || xx as usize >= level.width_tiles
                    || yy as usize >= level.height_tiles
                {
                    continue;
                }
                let index = yy as usize * level.width_tiles + xx as usize;
                self.patches.insert(
                    index,
                    TilePatch {
                        tile: patch.tiles[y * patch.width + x],
                        palette: level.palettes[index],
                        collision: patch.collision,
                    },
                );
            }
        }
    }
}

pub trait CollisionField {
    fn terrain_at(&self, point: Vec2) -> u8;

    fn traversable(&self, point: Vec2, marsh: bool) -> bool {
        let terrain = self.terrain_at(point);
        terrain == 0 || (marsh && terrain == 3)
    }

    fn clear_for_jeep(&self, point: Vec2, marsh: bool) -> bool {
        [
            Vec2::new(-4.0, -6.0),
            Vec2::new(4.0, -6.0),
            Vec2::new(-4.0, 6.0),
            Vec2::new(4.0, 6.0),
        ]
        .iter()
        .all(|offset| self.traversable(point + *offset, marsh))
    }
}

impl CollisionField for Level {
    fn terrain_at(&self, point: Vec2) -> u8 {
        if !point.x.is_finite() || !point.y.is_finite() {
            return 1;
        }
        Level::terrain_at(self, point)
    }
}

pub struct TerrainView<'a> {
    pub level: &'a Level,
    pub changes: &'a TerrainChanges,
}

impl CollisionField for TerrainView<'_> {
    fn terrain_at(&self, point: Vec2) -> u8 {
        if !point.x.is_finite()
            || !point.y.is_finite()
            || point.x < 0.0
            || point.y < 0.0
            || point.x >= self.level.width()
            || point.y >= self.level.height()
        {
            return 1;
        }
        let index = (point.y as usize / self.level.tile_size) * self.level.width_tiles
            + point.x as usize / self.level.tile_size;
        if let Some(patch) = self.changes.patches.get(&index) {
            patch.collision
        } else {
            self.level.collision[index]
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::assets::BossDefinition;

    fn level() -> Level {
        Level {
            name: "Destruction test".into(),
            width_tiles: 5,
            height_tiles: 5,
            tile_size: 8,
            tiles: vec![7; 25],
            palettes: vec![2; 25],
            collision: vec![0; 25],
            events: vec![],
            commands: vec![],
            boss: BossDefinition {
                kind: "test".into(),
                distance: 0.0,
            },
            initial_camera_from_bottom: 0,
            start_camera_x: 0.0,
        }
    }

    #[test]
    fn patch_collision_matches_original_packed_ram_writes() {
        let captures: serde_json::Value =
            serde_json::from_str(include_str!("../tests/fixtures/patch_collision.json")).unwrap();
        let assets =
            crate::assets::Assets::load(&crate::assets::Assets::default_directory()).unwrap();
        for capture in captures.as_array().unwrap() {
            let patch = &assets.boss_patches[capture["patch"].as_u64().unwrap() as usize];
            let cells = capture["cells"].as_array().unwrap();
            assert_eq!(cells.len(), patch.width * patch.height);
            assert!(cells
                .iter()
                .all(|cell| cell["collision"].as_u64() == Some(u64::from(patch.collision))));
        }
    }
    #[test]
    fn prison_rubble_uses_original_tiles_and_preserves_each_palette() {
        let assets =
            crate::assets::Assets::load(&crate::assets::Assets::default_directory()).unwrap();
        let mut level = level();
        level.tiles.fill(99);
        level.collision.fill(1);
        for (index, palette) in level.palettes.iter_mut().enumerate() {
            *palette = (index % 4) as u8;
        }
        let mut changes = TerrainChanges::default();
        // Stage one's right half starts at the actor X, twelve pixels above Y.
        let patch = assets.destruction(0, 0x13).unwrap();
        changes.destroy_building(&level, Vec2::new(8.0, 20.0), patch);
        let expected = [
            77, 75, 96, 99, 78, 80, 85, 100, 79, 34, 34, 34, 75, 75, 86, 100,
        ];
        for y in 0..4 {
            for x in 0..4 {
                let index = (y + 1) * 5 + x + 1;
                assert_eq!(changes.patches[&index].tile, expected[y * 4 + x]);
                assert_eq!(changes.patches[&index].palette, level.palettes[index]);
                assert_eq!(
                    TerrainView {
                        level: &level,
                        changes: &changes
                    }
                    .terrain_at(Vec2::new((x + 1) as f32 * 8.0, (y + 1) as f32 * 8.0)),
                    1
                );
            }
        }
        assert_eq!(changes.patches.len(), 16);
        assert!(!changes.patches.contains_key(&0));
        let before = changes.patches.clone();
        changes.destroy_building(&level, Vec2::new(8.0, 20.0), patch);
        assert_eq!(changes.patches, before);
    }

    #[test]
    fn every_prison_variant_has_stage_specific_rubble_and_clips_to_map() {
        let assets =
            crate::assets::Assets::load(&crate::assets::Assets::default_directory()).unwrap();
        for stage in 0..6 {
            for kind in [0x13, 0x14, 0x15, 0x19, 0x1c, 0x1d] {
                let definition = assets.destruction(stage, kind).unwrap();
                assert_eq!((definition.width, definition.height), (4, 4));
                let mut changes = TerrainChanges::default();
                changes.destroy_building(&level(), Vec2::new(0.0, 0.0), definition);
                assert!(changes.patches.len() <= 16);
                assert!(changes.patches.keys().all(|&index| index < 25));
            }
        }
        assert_ne!(
            assets.destruction(0, 0x13).unwrap().tiles,
            assets.destruction(2, 0x13).unwrap().tiles
        );
        assert_eq!(assets.destruction(0, 0x14).unwrap().offset_x, -32);
        assert_eq!(assets.destruction(0, 0x13).unwrap().offset_x, 0);
    }
    #[test]
    fn background_patch_collision_is_independent_of_graphic_tiles() {
        let mut level = level();
        level.collision.fill(3);
        let mut changes = TerrainChanges::default();
        // DF93 reads the byte after tile data; E27D stores it separately in
        // packed collision RAM. The same graphic can have any collision class.
        for collision in 0..=3 {
            let patch = crate::world_art::BossTilePatch {
                id: 0,
                width: 1,
                height: 1,
                offset_x: 0,
                offset_y: 0,
                tiles: vec![99],
                collision,
            };
            changes.apply_boss_patch(&level, Vec2::new(16., 16.), &patch);
            let view = TerrainView {
                level: &level,
                changes: &changes,
            };
            assert_eq!(view.terrain_at(Vec2::new(16., 16.)), collision);
            assert_eq!(view.terrain_at(Vec2::new(24., 16.)), 3);
            assert_eq!(changes.patches[&12].tile, 99);
        }
    }
    #[test]
    fn recovered_patches_clip_repeat_and_clear_without_mutating_base() {
        let level = level();
        let patch = crate::world_art::BossTilePatch {
            id: 0,
            width: 2,
            height: 2,
            offset_x: 0,
            offset_y: 0,
            tiles: vec![1, 2, 3, 4],
            collision: 2,
        };
        let mut changes = TerrainChanges::default();
        changes.apply_boss_patch(&level, Vec2::new(-8., -8.), &patch);
        assert_eq!(changes.patches.len(), 1);
        assert_eq!(changes.patches[&0].tile, 4);
        assert_eq!(changes.patches[&0].collision, 2);
        let once = changes.patches.clone();
        changes.apply_boss_patch(&level, Vec2::new(-8., -8.), &patch);
        changes.apply_boss_patch(&level, Vec2::new(f32::NAN, 0.), &patch);
        assert_eq!(changes.patches, once);
        assert_eq!(level.tiles[0], 7);
        assert_eq!(level.collision[0], 0);
        changes.clear();
        assert!(changes.patches.is_empty());
    }
}
