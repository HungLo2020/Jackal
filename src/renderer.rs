//! Native raster drawing for exported PNG artwork and game-world coordinates.
//! The renderer knows nothing about the original console's video registers.

pub const WIDTH: usize = 256;
pub const HEIGHT: usize = 240;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

impl Rect {
    pub const fn new(x: i32, y: i32, width: u32, height: u32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Camera {
    pub x: f32,
    pub y: f32,
}

impl Camera {
    pub fn world_to_screen(self, x: f32, y: f32) -> (i32, i32) {
        ((x - self.x).round() as i32, (y - self.y).round() as i32)
    }
}

/// Decoded PNG pixels, in straight RGBA byte order.
#[derive(Clone, Debug)]
pub struct Texture {
    pub width: usize,
    pub height: usize,
    rgba: Vec<u8>,
}

impl Texture {
    pub fn from_rgba(width: usize, height: usize, rgba: Vec<u8>) -> Result<Self, String> {
        let expected = width.checked_mul(height).and_then(|n| n.checked_mul(4));
        if width == 0 || height == 0 || expected != Some(rgba.len()) {
            return Err("texture dimensions do not match RGBA pixels".into());
        }
        Ok(Self {
            width,
            height,
            rgba,
        })
    }

    pub fn rgba(&self) -> &[u8] {
        &self.rgba
    }
    pub fn bounds(&self) -> Rect {
        Rect::new(0, 0, self.width as u32, self.height as u32)
    }
}

pub struct Canvas {
    pub width: usize,
    pub height: usize,
    pixels: Vec<u32>,
}

impl Default for Canvas {
    fn default() -> Self {
        Self::new(WIDTH, HEIGHT)
    }
}

impl Canvas {
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            width,
            height,
            pixels: vec![0; width * height],
        }
    }

    /// Packed 0x00RRGGBB pixels, directly consumable by minifb.
    pub fn pixels(&self) -> &[u32] {
        &self.pixels
    }
    pub fn clear(&mut self, color: u32) {
        self.pixels.fill(color & 0xffffff);
    }

    pub fn pixel(&mut self, x: i32, y: i32, color: u32) {
        if x >= 0 && y >= 0 && (x as usize) < self.width && (y as usize) < self.height {
            self.pixels[y as usize * self.width + x as usize] = color & 0xffffff;
        }
    }

    pub fn fill_rect(&mut self, rect: Rect, color: u32) {
        let x0 = (rect.x as i64).clamp(0, self.width as i64) as usize;
        let y0 = (rect.y as i64).clamp(0, self.height as i64) as usize;
        let x1 = (rect.x as i64 + rect.width as i64).clamp(0, self.width as i64) as usize;
        let y1 = (rect.y as i64 + rect.height as i64).clamp(0, self.height as i64) as usize;
        for y in y0..y1 {
            self.pixels[y * self.width + x0..y * self.width + x1].fill(color & 0xffffff);
        }
    }

    pub fn outline(&mut self, rect: Rect, color: u32) {
        if rect.width == 0 || rect.height == 0 {
            return;
        }
        self.fill_rect(Rect::new(rect.x, rect.y, rect.width, 1), color);
        self.fill_rect(
            Rect::new(rect.x, rect.y + rect.height as i32 - 1, rect.width, 1),
            color,
        );
        self.fill_rect(Rect::new(rect.x, rect.y, 1, rect.height), color);
        self.fill_rect(
            Rect::new(rect.x + rect.width as i32 - 1, rect.y, 1, rect.height),
            color,
        );
    }

    pub fn draw_texture(&mut self, texture: &Texture, x: i32, y: i32) {
        self.draw_region(
            texture,
            texture.bounds(),
            Rect::new(x, y, texture.width as u32, texture.height as u32),
            false,
            false,
        );
    }

    /// Nearest-neighbor scaling; transparent source pixels leave the destination
    /// untouched. Flips are applied within the selected atlas rectangle.
    pub fn draw_region(
        &mut self,
        texture: &Texture,
        source: Rect,
        dest: Rect,
        flip_x: bool,
        flip_y: bool,
    ) {
        if source.width == 0 || source.height == 0 || dest.width == 0 || dest.height == 0 {
            return;
        }
        let x0 = (dest.x as i64).clamp(0, self.width as i64);
        let y0 = (dest.y as i64).clamp(0, self.height as i64);
        let x1 = (dest.x as i64 + dest.width as i64).clamp(0, self.width as i64);
        let y1 = (dest.y as i64 + dest.height as i64).clamp(0, self.height as i64);
        for y in y0..y1 {
            let local_y = (y - dest.y as i64) * source.height as i64 / dest.height as i64;
            let sy = source.y as i64
                + if flip_y {
                    source.height as i64 - 1 - local_y
                } else {
                    local_y
                };
            if sy < 0 || sy >= texture.height as i64 {
                continue;
            }
            for x in x0..x1 {
                let local_x = (x - dest.x as i64) * source.width as i64 / dest.width as i64;
                let sx = source.x as i64
                    + if flip_x {
                        source.width as i64 - 1 - local_x
                    } else {
                        local_x
                    };
                if sx < 0 || sx >= texture.width as i64 {
                    continue;
                }
                let i = (sy as usize * texture.width + sx as usize) * 4;
                let alpha = texture.rgba[i + 3] as u32;
                if alpha == 0 {
                    continue;
                }
                let p = y as usize * self.width + x as usize;
                let rgb = ((texture.rgba[i] as u32) << 16)
                    | ((texture.rgba[i + 1] as u32) << 8)
                    | texture.rgba[i + 2] as u32;
                self.pixels[p] = if alpha == 255 {
                    rgb
                } else {
                    blend(rgb, self.pixels[p], alpha)
                };
            }
        }
    }

    #[allow(clippy::too_many_arguments)] // Explicit raster geometry and transform parameters.
    pub fn draw_world_region(
        &mut self,
        texture: &Texture,
        source: Rect,
        x: f32,
        y: f32,
        camera: Camera,
        flip_x: bool,
        flip_y: bool,
    ) {
        let (x, y) = camera.world_to_screen(x, y);
        self.draw_region(
            texture,
            source,
            Rect::new(x, y, source.width, source.height),
            flip_x,
            flip_y,
        );
    }

    /// Draw a row-major tile layer using an atlas with equally sized tiles.
    /// An optional empty ID allows decorative layers with transparent holes.
    pub fn tile_layer(
        &mut self,
        atlas: &Texture,
        tiles: &[u16],
        columns: usize,
        tile_size: u32,
        camera: Camera,
        empty: Option<u16>,
    ) {
        if columns == 0 || tile_size == 0 {
            return;
        }
        let atlas_columns = atlas.width / tile_size as usize;
        if atlas_columns == 0 {
            return;
        }
        let rows = tiles.len().div_ceil(columns);
        let first_x = (camera.x.floor() as i32)
            .div_euclid(tile_size as i32)
            .max(0) as usize;
        let first_y = (camera.y.floor() as i32)
            .div_euclid(tile_size as i32)
            .max(0) as usize;
        let end_x = (((camera.x + self.width as f32).ceil() as i32).div_euclid(tile_size as i32)
            + 1)
        .max(0) as usize;
        let end_y = (((camera.y + self.height as f32).ceil() as i32).div_euclid(tile_size as i32)
            + 1)
        .max(0) as usize;
        for y in first_y..end_y.min(rows) {
            for x in first_x..end_x.min(columns) {
                let Some(&id) = tiles.get(y * columns + x) else {
                    continue;
                };
                if empty == Some(id) {
                    continue;
                }
                let source = Rect::new(
                    (id as usize % atlas_columns * tile_size as usize) as i32,
                    (id as usize / atlas_columns * tile_size as usize) as i32,
                    tile_size,
                    tile_size,
                );
                self.draw_world_region(
                    atlas,
                    source,
                    (x * tile_size as usize) as f32,
                    (y * tile_size as usize) as f32,
                    camera,
                    false,
                    false,
                );
            }
        }
    }

    pub fn text(&mut self, x: i32, y: i32, text: &str, color: u32, scale: u32) {
        if scale == 0 {
            return;
        }
        let mut cursor_x = x;
        let mut cursor_y = y;
        for ch in text.chars() {
            if ch == '\n' {
                cursor_x = x;
                cursor_y += 8 * scale as i32;
                continue;
            }
            for (row, bits) in glyph(ch.to_ascii_uppercase()).iter().enumerate() {
                for col in 0..5 {
                    if bits & (1 << (4 - col)) != 0 {
                        self.fill_rect(
                            Rect::new(
                                cursor_x + col * scale as i32,
                                cursor_y + row as i32 * scale as i32,
                                scale,
                                scale,
                            ),
                            color,
                        );
                    }
                }
            }
            cursor_x += 6 * scale as i32;
        }
    }

    pub fn centered_text(&mut self, y: i32, text: &str, color: u32, scale: u32) {
        let width = text.chars().count() as i32 * 6 * scale as i32 - scale as i32;
        self.text((self.width as i32 - width) / 2, y, text, color, scale);
    }
}

/// High-level scene rendering. Exported level artwork supplies the environment;
/// actors are assembled from named sprite frames and world-space positions.
struct SceneSprite {
    slot: u8,
    frame: u16,
    x: f32,
    y: f32,
    camera: Camera,
    palette: Option<u8>,
    flip_x: bool,
    flip_y: bool,
}
impl SceneSprite {
    #[allow(clippy::too_many_arguments)]
    fn new(
        slot: u8,
        frame: u16,
        x: f32,
        y: f32,
        camera: Camera,
        palette: Option<u8>,
        flip_x: bool,
        flip_y: bool,
    ) -> Self {
        Self {
            slot,
            frame,
            x,
            y,
            camera,
            palette,
            flip_x,
            flip_y,
        }
    }
}
fn scene_part_priority(rotation: u8, part_index: usize) -> u8 {
    rotation.wrapping_add((part_index as u8).wrapping_mul(49)) & 63
}

struct ScenePart<'a> {
    priority: u8,
    texture: &'a Texture,
    source: Rect,
    destination: Rect,
    flip_x: bool,
    flip_y: bool,
    behind_background: bool,
}

/// A high-priority sprite behind opaque scenery also hides lower-priority
/// sprites; restoring the original background implements that composition rule.
fn restore_opaque_background(canvas: &mut Canvas, part: &ScenePart<'_>, background: &[u32]) {
    let dest = part.destination;
    let source = part.source;
    for dy in 0..dest.height as i32 {
        let y = dest.y + dy;
        if y < 0 || y >= canvas.height as i32 {
            continue;
        }
        let sy = dy as u32 * source.height / dest.height;
        let sy = source.y
            + if part.flip_y {
                source.height - 1 - sy
            } else {
                sy
            } as i32;
        for dx in 0..dest.width as i32 {
            let x = dest.x + dx;
            if x < 0 || x >= canvas.width as i32 {
                continue;
            }
            let sx = dx as u32 * source.width / dest.width;
            let sx = source.x
                + if part.flip_x {
                    source.width - 1 - sx
                } else {
                    sx
                } as i32;
            if sx < 0
                || sy < 0
                || sx >= part.texture.width as i32
                || sy >= part.texture.height as i32
            {
                continue;
            }
            let alpha = part.texture.rgba[(sy as usize * part.texture.width + sx as usize) * 4 + 3];
            let pixel = y as usize * canvas.width + x as usize;
            if alpha != 0 && background[pixel] != 0 {
                canvas.pixels[pixel] = background[pixel];
            }
        }
    }
}

pub struct Renderer {
    pub canvas: Canvas,
    progress: f32,
    phase_age: u64,
    headquarters_tick: Option<u16>,
    river_frame: Option<(bool, u64)>,
    stage_three_water_frame: Option<(bool, (u8, usize))>,
    headquarters_final: Option<u16>,
    headquarters_building: Option<(Option<u8>, Option<u8>, u8, u16)>,
}

impl Default for Renderer {
    fn default() -> Self {
        Self::new()
    }
}

impl Renderer {
    pub fn new() -> Self {
        Self {
            canvas: Canvas::default(),
            progress: 0.,
            phase_age: 0,
            headquarters_tick: None,
            river_frame: None,
            stage_three_water_frame: None,
            headquarters_final: None,
            headquarters_building: None,
        }
    }

    fn world_textures<'a>(
        &self,
        assets: &'a crate::assets::Assets,
        stage: usize,
    ) -> (&'a Texture, &'a Texture) {
        if let Some(tick) = self.headquarters_final {
            return assets.headquarters_art.final_textures(tick);
        }
        if let Some((prior, previous, current, elapsed)) = self.headquarters_building {
            return assets
                .headquarters_art
                .building_textures(prior, previous, current, elapsed);
        }
        if let Some(tick) = self.headquarters_tick {
            return assets.headquarters_art.textures(tick);
        }
        if let Some((palette, age)) = self.river_frame {
            return assets.river_art.textures(palette, age);
        }
        // The reference palette-change export also contains subsequent water
        // uploads. Stop that captured sequence after the palette transfer;
        // the native water scheduler owns the following animation.
        let age = if stage == 2 && self.progress >= 2739. {
            self.phase_age.min(26)
        } else {
            self.phase_age
        };
        let textures = assets.textures(stage, self.progress, age);
        if let Some((late, frame)) = self.stage_three_water_frame {
            return (assets.stage_three_water_art.tiles(late, frame), textures.1);
        }
        textures
    }

    pub fn render(&mut self, game: &crate::game::Game, assets: &crate::assets::Assets) -> &[u32] {
        use crate::game::GameState;
        self.canvas.clear(0);
        if let Some((clip, tick)) = game.presentation(assets) {
            clip.draw(&mut self.canvas, tick);
            if matches!(game.state, GameState::StageClear { .. })
                || (matches!(game.state, GameState::StageLoading { ticks: 0 })
                    && !game.loading_continue)
            {
                assets.scoreboard.draw(
                    &mut self.canvas,
                    &[game.players[0].score, game.players[1].score],
                    &[game.players[0].delivered, game.players[1].delivered],
                    game.selected_players as u8,
                    if matches!(game.state, GameState::StageLoading { .. }) {
                        568
                    } else {
                        tick
                    },
                );
            }
            return self.canvas.pixels();
        }
        if matches!(game.state, GameState::Title) {
            assets
                .cinematics
                .get(if game.selected_players == 2 {
                    "title_2p"
                } else {
                    "title_1p"
                })
                .expect("title asset")
                .draw(&mut self.canvas, 0);
            return self.canvas.pixels();
        }
        let stage = game.stage.min(assets.levels.len() - 1);
        let level = &assets.levels[stage];
        self.progress = game.texture_phase_distance;
        self.river_frame = game.river_animation_frame();
        self.stage_three_water_frame = if game.stage == 2 {
            game.stage_three_water
                .frame()
                .map(|frame| (game.texture_phase_distance >= 2739., frame))
        } else {
            None
        };
        self.headquarters_tick = game
            .boss
            .as_ref()
            .and_then(|b| b.headquarters_transition_tick());
        self.headquarters_final = game
            .boss
            .as_ref()
            .and_then(|b| b.headquarters_final_death_tick());
        self.headquarters_building = game
            .boss
            .as_ref()
            .and_then(|b| b.headquarters_building_graphics());
        self.phase_age = game.tick.saturating_sub(game.texture_phase_tick);
        let source_atlas = self.world_textures(assets, stage).0;
        let cycled_atlas = game
            .palette_cycle
            .visible()
            .filter(|_| game.texture_phase_distance == 0.)
            .map(|phase| assets.palette_cycle_art.compose(source_atlas, stage, phase));
        let atlas = cycled_atlas.as_ref().unwrap_or(source_atlas);
        // Palette variants occupy consecutive groups of 256 atlas tiles.
        let first_x = ((game.camera.x / level.tile_size as f32).floor() as i32).max(0) as usize;
        let first_y = ((game.camera.y / level.tile_size as f32).floor() as i32).max(0) as usize;
        let count_x = WIDTH / level.tile_size + 2;
        let count_y = HEIGHT / level.tile_size + 2;
        for y in first_y..(first_y + count_y).min(level.height_tiles) {
            for x in first_x..(first_x + count_x).min(level.width_tiles) {
                let i = y * level.width_tiles + x;
                let (tile_id, palette) = game
                    .terrain
                    .patches
                    .get(&i)
                    .map(|patch| (patch.tile, patch.palette))
                    .unwrap_or((level.tiles[i], level.palettes[i]));
                let tile = tile_id as usize + palette as usize * 256;
                let columns = atlas.width / level.tile_size;
                let source = Rect::new(
                    (tile % columns * level.tile_size) as i32,
                    (tile / columns * level.tile_size) as i32,
                    level.tile_size as u32,
                    level.tile_size as u32,
                );
                self.canvas.draw_world_region(
                    atlas,
                    source,
                    (x * level.tile_size) as f32,
                    (y * level.tile_size) as f32,
                    game.camera,
                    false,
                    false,
                );
            }
        }
        let mut scene = Vec::new();
        for death in &game.infantry_deaths {
            let draw_slot = death.actor_handle.map_or(0, |h| h.slot());
            if let Some(frame) = death.frame(assets) {
                scene.push(SceneSprite::new(
                    draw_slot,
                    frame,
                    death.position.x,
                    death.position.y,
                    game.camera,
                    Some(1),
                    false,
                    false,
                ));
            }
        }
        for death in &game.enemy_deaths {
            let draw_slot = death.actor_handle.map_or(0, |h| h.slot());
            if death.alive && death.sprite != 0 {
                scene.push(SceneSprite::new(
                    draw_slot,
                    death.sprite,
                    death.position.x,
                    death.position.y,
                    game.camera,
                    Some(death.palette),
                    death.flips.0,
                    death.flips.1,
                ));
            }
        }
        for enemy in &game.enemies {
            let draw_slot = enemy.actor_handle.map_or(0, |h| h.slot());
            if !enemy.alive {
                continue;
            }
            if let Some(frame) = enemy.sprite(assets) {
                scene.push(SceneSprite::new(
                    draw_slot,
                    frame,
                    enemy.position.x,
                    enemy.position.y,
                    game.camera,
                    enemy.palette_override(),
                    enemy.sprite_flips().0,
                    enemy.sprite_flips().1,
                ));
            }
        }
        if let Some(boss) = &game.boss {
            for part in &boss.parts {
                let draw_slot = part.actor_handle.map_or(0, |h| h.slot());
                if part.active {
                    if let Some(id) = part.background_patch {
                        let patch = &assets.boss_patches[usize::from(id)];
                        let x0 = ((part.position[0] + patch.offset_x as f32) / 8.).floor() as i32;
                        let y0 = ((part.position[1] + patch.offset_y as f32) / 8.).floor() as i32;
                        // Terrain and boss patches share the same animated background atlas.
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
                                let palette = usize::from(
                                    level.palettes[yy as usize * level.width_tiles + xx as usize],
                                );
                                let tile =
                                    usize::from(patch.tiles[y * patch.width + x]) + palette * 256;
                                self.canvas.draw_world_region(
                                    atlas,
                                    Rect::new((tile % 16 * 8) as i32, (tile / 16 * 8) as i32, 8, 8),
                                    xx as f32 * 8.,
                                    yy as f32 * 8.,
                                    game.camera,
                                    false,
                                    false,
                                );
                            }
                        }
                    }
                }
                if (!part.alive && part.death_ticks == 0) || !part.active {
                    continue;
                }
                scene.push(SceneSprite::new(
                    draw_slot,
                    part.sprite,
                    part.position[0],
                    part.position[1],
                    game.camera,
                    part.palette,
                    part.flip_x,
                    part.flip_y,
                ));
            }
        }
        for child in &game.boss_children {
            let draw_slot = child.actor_handle.map_or(0, |h| h.slot());
            if child.alive && child.sprite != 0 {
                scene.push(SceneSprite::new(
                    draw_slot,
                    child.sprite,
                    child.position.x,
                    child.position.y,
                    game.camera,
                    if child.death_ticks > 0 { Some(3) } else { None },
                    child.flips.0,
                    child.flips.1,
                ));
            }
        }
        for heli in &game.rescue_helicopters {
            let draw_slot = heli.actor_handle.map_or(0, |h| h.slot());
            if heli.sprite != 0 {
                scene.push(SceneSprite::new(
                    draw_slot,
                    heli.sprite as u16,
                    heli.position.x,
                    heli.position.y,
                    game.camera,
                    None,
                    false,
                    heli.flip_y,
                ));
            }
        }
        for release in &game.prison_releases {
            let draw_slot = release.actor_handle.map_or(0, |h| h.slot());
            if let Some((sprite, palette)) = release.sprite() {
                scene.push(SceneSprite::new(
                    draw_slot,
                    sprite,
                    release.position.x,
                    release.position.y,
                    game.camera,
                    Some(palette),
                    false,
                    false,
                ));
            }
        }
        for delivery in &game.deliveries {
            let draw_slot = delivery.actor_handle.map_or(0, |h| h.slot());
            scene.push(SceneSprite::new(
                draw_slot,
                delivery.sprite(),
                delivery.position.x,
                delivery.position.y,
                game.camera,
                Some(delivery.palette),
                delivery.left,
                false,
            ));
        }
        for prisoner in &game.prisoners {
            let draw_slot = prisoner.actor_handle.map_or(0, |h| h.slot());
            let frame = prisoner.sprite;
            if frame == 0 {
                continue;
            }
            scene.push(SceneSprite::new(
                draw_slot,
                frame,
                prisoner.position.x,
                prisoner.position.y,
                game.camera,
                Some(prisoner.palette),
                prisoner.flip_x,
                false,
            ));
        }
        for object in &game.background_objects {
            let draw_slot = object.actor_handle.map_or(0, |h| h.slot());
            if let Some(sprite) = object.sprite() {
                scene.push(SceneSprite::new(
                    draw_slot,
                    sprite,
                    object.position.x,
                    object.position.y,
                    game.camera,
                    Some(3),
                    false,
                    false,
                ));
            }
        }
        for mine in &game.mines {
            let draw_slot = mine.actor_handle.map_or(0, |h| h.slot());
            if mine.sprite != 0 {
                scene.push(SceneSprite::new(
                    draw_slot,
                    mine.sprite,
                    mine.position.x,
                    mine.position.y,
                    game.camera,
                    Some(mine.palette),
                    false,
                    false,
                ));
            }
        }
        for pickup in &game.pickups {
            let draw_slot = pickup.actor_handle.map_or(0, |h| h.slot());
            if !pickup.visible() {
                continue;
            }
            scene.push(SceneSprite::new(
                draw_slot,
                46,
                pickup.position.x,
                pickup.position.y,
                game.camera,
                Some(pickup.palette),
                false,
                false,
            ));
        }
        for actor in &game.environment_actors {
            let draw_slot = actor.actor_handle.map_or(0, |h| h.slot());
            if actor.alive && actor.sprite != 0 {
                scene.push(SceneSprite::new(
                    draw_slot,
                    actor.sprite as u16,
                    actor.position.x,
                    actor.position.y,
                    game.camera,
                    Some(actor.palette),
                    false,
                    false,
                ));
            }
        }
        for (index, player) in game.players.iter().enumerate() {
            let draw_slot = 16 + index as u8;
            if player.respawn_ticks > 0 {
                let age = crate::player::DEATH_TICKS - player.respawn_ticks;
                let clip = &assets.animations["jeep_death"];
                if let Some(&frame) = clip.frames.get(usize::from(age / clip.frame_ticks)) {
                    scene.push(SceneSprite::new(
                        draw_slot,
                        frame,
                        player.position.x,
                        player.position.y,
                        game.camera,
                        Some(3),
                        false,
                        false,
                    ));
                }
                continue;
            }
            if !player.active() || (player.invulnerable > 0 && game.tick % 6 < 3) {
                continue;
            }
            let (name, flip_x, flip_y) = player.animation();
            let frame = assets.animation_frame(name, game.tick);
            scene.push(SceneSprite::new(
                draw_slot,
                frame,
                player.position.x,
                player.position.y,
                game.camera,
                if index == 1 { Some(1) } else { None },
                flip_x,
                flip_y,
            ));
        }
        for shot in &game.projectiles {
            let draw_slot = shot
                .actor_handle
                .map(|h| h.slot())
                .or(shot.player_slot)
                .unwrap_or(24);
            let (frame, palette) = if shot.has_native_player_state() {
                (shot.player_sprite(), shot.player_palette())
            } else if shot.has_native_enemy_state() {
                (shot.enemy_sprite(), Some(shot.enemy_palette()))
            } else {
                continue;
            };
            if shot.remaining == 0 {
                continue;
            }
            if let Some(frame) = frame {
                let (flip_x, flip_y) = shot.sprite_flips();
                scene.push(SceneSprite::new(
                    draw_slot,
                    frame,
                    shot.position.x,
                    shot.position.y,
                    game.camera,
                    palette,
                    flip_x,
                    flip_y,
                ));
            }
        }
        for explosion in &game.explosions {
            let draw_slot = explosion.actor_handle.map_or(0, |h| h.slot());
            let frame = 25 + (24u16.saturating_sub(explosion.remaining) / 8).min(2);
            scene.push(SceneSprite::new(
                draw_slot,
                frame,
                explosion.position.x,
                explosion.position.y,
                game.camera,
                None,
                false,
                false,
            ));
        }
        self.draw_scene(assets, game, stage, scene);
        match game.state {
            GameState::Briefing { .. } => {
                self.panel(
                    "MISSION",
                    &format!("{}  {}", stage + 1, level.name.to_uppercase()),
                    "RESCUE THE PRISONERS",
                );
            }
            GameState::StageClear { .. } => self.panel(
                "MISSION COMPLETE",
                "PRISONERS RESCUED",
                "ADVANCING TO NEXT AREA",
            ),
            GameState::GameOver { .. } => self.panel("GAME OVER", "", ""),
            GameState::Continue { ticks } => {
                self.panel("CONTINUE?", &format!("{}", ticks / 60), "ENTER CONTINUE")
            }
            GameState::Ending { .. } => {
                self.canvas.clear(0);
                self.canvas
                    .centered_text(52, "MISSION ACCOMPLISHED", 0xffd59a, 1);
                self.canvas
                    .centered_text(85, "ALL SIX AREAS LIBERATED", 0xffffff, 1);
                self.canvas
                    .centered_text(110, "YOUR COMRADES ARE FREE", 0xffffff, 1);
                self.canvas
                    .centered_text(153, "THANK YOU FOR PLAYING", 0xb8b8b8, 1);
                self.canvas
                    .centered_text(193, "ENTER TO RETURN", 0xffffff, 1);
            }
            _ => {}
        }
        if game.paused {
            self.panel("PAUSED", "", "P TO RESUME");
        }
        self.canvas.pixels()
    }

    fn draw_scene(
        &mut self,
        assets: &crate::assets::Assets,
        game: &crate::game::Game,
        stage: usize,
        mut sprites: Vec<SceneSprite>,
    ) {
        // Bank1 8008/8019: actors31..0; 81EA advances OAM by49 slots.
        sprites.sort_by_key(|sprite| std::cmp::Reverse(sprite.slot));
        let atlas = self.world_textures(assets, stage).1;
        let columns = atlas.width / 8;
        let mut parts = Vec::new();
        let rotation = game.sprite_rotation;
        for sprite in sprites {
            let Some(frame) = assets.frames.get(usize::from(sprite.frame)) else {
                continue;
            };
            let (origin_x, origin_y) = sprite.camera.world_to_screen(sprite.x, sprite.y);
            for part in &frame.parts {
                let dx = if sprite.flip_x { -part.x - 8 } else { part.x };
                let dy = if sprite.flip_y { -part.y - 16 } else { part.y };
                let x = origin_x + dx;
                let y = origin_y + dy;
                // Original81A3/81D7 skips coordinates that wrap the byte domain.
                if !(0..256).contains(&x) || !(0..256).contains(&y) || parts.len() == 64 {
                    continue;
                }
                let palette = usize::from(part.palette | sprite.palette.unwrap_or(0));
                let tile = usize::from(part.tile) + palette * 256;
                parts.push(ScenePart {
                    priority: scene_part_priority(rotation, parts.len()),
                    texture: atlas,
                    source: Rect::new(
                        (tile % columns * 8) as i32,
                        (tile / columns * 16) as i32,
                        8,
                        16,
                    ),
                    destination: Rect::new(x, y + 1, 8, 16),
                    flip_x: part.flip_x ^ sprite.flip_x,
                    flip_y: part.flip_y ^ sprite.flip_y,
                    behind_background: part.behind_background,
                });
            }
        }
        for hud in assets.hud.pieces(&game.players, game.tick) {
            if parts.len() == 64 {
                break;
            }
            parts.push(ScenePart {
                priority: scene_part_priority(rotation, parts.len()),
                texture: hud.texture,
                source: hud.source,
                destination: hud.destination,
                behind_background: false,
                flip_x: false,
                flip_y: false,
            });
        }
        // Only the airbase formation frames carry background priority. The
        // original palette proof verifies that black is exactly color index0
        // throughout that stage, including its complete graphics transition.
        let background = parts
            .iter()
            .any(|part| part.behind_background)
            .then(|| self.canvas.pixels.clone());
        parts.sort_by_key(|part| std::cmp::Reverse(part.priority));
        for part in parts {
            self.canvas.draw_region(
                part.texture,
                part.source,
                part.destination,
                part.flip_x,
                part.flip_y,
            );
            if part.behind_background {
                restore_opaque_background(&mut self.canvas, &part, background.as_ref().unwrap());
            }
        }
    }

    /// Assemble the original sprite parts with their own flips and palette.
    /// Whole-frame mirroring also reflects part positions around the actor.
    #[allow(clippy::too_many_arguments)] // Metasprite placement and palette transform.
    pub fn sprite(
        &mut self,
        assets: &crate::assets::Assets,
        stage: usize,
        frame: u16,
        x: f32,
        y: f32,
        camera: Camera,
        palette_override: Option<u8>,
        flip_x: bool,
        flip_y: bool,
    ) {
        let Some(frame) = assets.frames.get(frame as usize) else {
            return;
        };
        let atlas = self.world_textures(assets, stage).1;
        let columns = atlas.width / 8;
        let (origin_x, origin_y) = camera.world_to_screen(x, y);
        for part in frame.parts.iter().rev() {
            let palette = (part.palette | palette_override.unwrap_or(0)) as usize;
            let tile = part.tile as usize + palette * 256;
            let source = Rect::new(
                (tile % columns * 8) as i32,
                (tile / columns * 16) as i32,
                8,
                16,
            );
            let dx = if flip_x { -part.x - 8 } else { part.x };
            let dy = if flip_y { -part.y - 16 } else { part.y };
            self.canvas.draw_region(
                atlas,
                source,
                Rect::new(origin_x + dx, origin_y + dy + 1, 8, 16),
                part.flip_x ^ flip_x,
                part.flip_y ^ flip_y,
            );
        }
    }

    fn panel(&mut self, heading: &str, detail: &str, prompt: &str) {
        self.canvas.fill_rect(Rect::new(9, 85, 238, 75), 0x080810);
        self.canvas.outline(Rect::new(9, 85, 238, 75), 0x8a8a8a);
        self.canvas.centered_text(96, heading, 0xffd59a, 1);
        self.canvas.centered_text(117, detail, 0xffffff, 1);
        self.canvas.centered_text(141, prompt, 0xb8b8b8, 1);
    }
}

fn blend(source: u32, dest: u32, alpha: u32) -> u32 {
    let channel = |shift: u32| {
        ((((source >> shift) & 255) * alpha + ((dest >> shift) & 255) * (255 - alpha) + 127) / 255)
            << shift
    };
    channel(16) | channel(8) | channel(0)
}

fn glyph(ch: char) -> [u8; 7] {
    match ch {
        '*' => [0, 21, 14, 31, 14, 21, 0],
        'A' => [14, 17, 17, 31, 17, 17, 17],
        'B' => [30, 17, 17, 30, 17, 17, 30],
        'C' => [14, 17, 16, 16, 16, 17, 14],
        'D' => [30, 17, 17, 17, 17, 17, 30],
        'E' => [31, 16, 16, 30, 16, 16, 31],
        'F' => [31, 16, 16, 30, 16, 16, 16],
        'G' => [14, 17, 16, 23, 17, 17, 15],
        'H' => [17, 17, 17, 31, 17, 17, 17],
        'I' => [14, 4, 4, 4, 4, 4, 14],
        'J' => [7, 2, 2, 2, 18, 18, 12],
        'K' => [17, 18, 20, 24, 20, 18, 17],
        'L' => [16, 16, 16, 16, 16, 16, 31],
        'M' => [17, 27, 21, 21, 17, 17, 17],
        'N' => [17, 25, 21, 19, 17, 17, 17],
        'O' => [14, 17, 17, 17, 17, 17, 14],
        'P' => [30, 17, 17, 30, 16, 16, 16],
        'Q' => [14, 17, 17, 17, 21, 18, 13],
        'R' => [30, 17, 17, 30, 20, 18, 17],
        'S' => [15, 16, 16, 14, 1, 1, 30],
        'T' => [31, 4, 4, 4, 4, 4, 4],
        'U' => [17, 17, 17, 17, 17, 17, 14],
        'V' => [17, 17, 17, 17, 17, 10, 4],
        'W' => [17, 17, 17, 21, 21, 21, 10],
        'X' => [17, 17, 10, 4, 10, 17, 17],
        'Y' => [17, 17, 10, 4, 4, 4, 4],
        'Z' => [31, 1, 2, 4, 8, 16, 31],
        '0' => [14, 17, 19, 21, 25, 17, 14],
        '1' => [4, 12, 4, 4, 4, 4, 14],
        '2' => [14, 17, 1, 2, 4, 8, 31],
        '3' => [30, 1, 1, 14, 1, 1, 30],
        '4' => [2, 6, 10, 18, 31, 2, 2],
        '5' => [31, 16, 16, 30, 1, 1, 30],
        '6' => [14, 16, 16, 30, 17, 17, 14],
        '7' => [31, 1, 2, 4, 8, 8, 8],
        '8' => [14, 17, 17, 14, 17, 17, 14],
        '9' => [14, 17, 17, 15, 1, 1, 14],
        ':' => [0, 4, 4, 0, 4, 4, 0],
        '.' => [0, 0, 0, 0, 0, 4, 4],
        '-' => [0, 0, 0, 31, 0, 0, 0],
        '+' => [0, 4, 4, 31, 4, 4, 0],
        '/' => [1, 2, 2, 4, 8, 8, 16],
        '!' => [4, 4, 4, 4, 4, 0, 4],
        '?' => [14, 17, 1, 2, 4, 0, 4],
        '(' => [2, 4, 8, 8, 8, 4, 2],
        ')' => [8, 4, 2, 2, 2, 4, 8],
        '=' => [0, 31, 0, 31, 0, 0, 0],
        _ => [0; 7],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn background_priority_assets_are_bound_to_the_verified_airbase_palette() {
        let assets =
            crate::assets::Assets::load(&crate::assets::Assets::default_directory()).unwrap();
        let frames: Vec<_> = assets
            .frames
            .iter()
            .filter(|frame| frame.parts.iter().any(|part| part.behind_background))
            .map(|frame| frame.id)
            .collect();
        assert_eq!(frames, vec![166, 167, 169, 170]);
        for (stage, level) in assets.levels.iter().enumerate() {
            if level.events.iter().any(|event| event.source_type == 50) {
                assert_eq!(stage, 3);
            }
        }
        let palette: Vec<String> =
            serde_json::from_str(include_str!("../assets/textures/stage_04_palette.json")).unwrap();
        assert_eq!(palette[0], "#000000");
        for index in (0..16).filter(|index| index % 4 != 0) {
            assert_ne!(palette[index], palette[0]);
        }
    }
    #[test]
    fn background_priority_hides_lower_sprites_only_at_opaque_sprite_pixels() {
        let texture =
            Texture::from_rgba(3, 1, vec![255, 0, 0, 255, 255, 0, 0, 255, 255, 0, 0, 0]).unwrap();
        let mut canvas = Canvas::new(3, 1);
        let background = [0, 0x005324, 0x005324];
        canvas.pixels.fill(0x0000ff); // A lower-priority foreground sprite.
        let part = ScenePart {
            priority: 0,
            texture: &texture,
            source: Rect::new(0, 0, 3, 1),
            destination: Rect::new(0, 0, 3, 1),
            flip_x: false,
            flip_y: false,
            behind_background: true,
        };
        canvas.draw_region(&texture, part.source, part.destination, false, false);
        restore_opaque_background(&mut canvas, &part, &background);
        assert_eq!(canvas.pixels(), &[0xff0000, 0x005324, 0x0000ff]);
    }
    #[test]
    fn scene_priority_matches_every_original_oam_rotation() {
        let captures: serde_json::Value =
            serde_json::from_str(include_str!("../tests/fixtures/oam_order.json")).unwrap();
        for capture in captures.as_array().unwrap() {
            let rotation = capture["after"].as_u64().unwrap() as u8;
            assert_eq!(
                rotation,
                (capture["before"].as_u64().unwrap() as u8 + 17) & 63
            );
            let entries = capture["entries"].as_array().unwrap();
            assert_eq!(entries.len(), 32);
            for entry in entries {
                let actor = (entry["x"].as_u64().unwrap() - 12) / 6;
                assert_eq!(
                    entry["priority"].as_u64().unwrap(),
                    u64::from(scene_part_priority(rotation, 31 - actor as usize))
                );
            }
        }
    }
    #[test]
    fn alpha_compositing_and_negative_clip() {
        let t = Texture::from_rgba(
            2,
            2,
            vec![
                255, 0, 0, 255, 0, 255, 0, 0, 0, 0, 255, 128, 255, 255, 255, 255,
            ],
        )
        .unwrap();
        let mut c = Canvas::new(2, 2);
        c.clear(0xff0000);
        c.draw_texture(&t, 0, 0);
        assert_eq!(c.pixels(), &[0xff0000, 0xff0000, 0x7f0080, 0xffffff]);
        c.draw_texture(&t, -1, -1);
        assert_eq!(c.pixels()[0], 0xffffff);
    }

    #[test]
    fn scaling_and_flip_select_correct_source_pixels() {
        let t = Texture::from_rgba(2, 1, vec![255, 0, 0, 255, 0, 255, 0, 255]).unwrap();
        let mut c = Canvas::new(4, 1);
        c.draw_region(&t, t.bounds(), Rect::new(0, 0, 4, 1), true, false);
        assert_eq!(c.pixels(), &[0x00ff00, 0x00ff00, 0xff0000, 0xff0000]);
    }

    #[test]
    fn text_scales_and_clips_without_touching_other_pixels() {
        let mut c = Canvas::new(16, 16);
        c.text(0, 0, "A", 0xffffff, 2);
        assert_eq!(c.pixels()[0], 0);
        assert_eq!(c.pixels()[2], 0xffffff);
        assert_eq!(c.pixels()[16 + 2], 0xffffff);
        c.text(-10, -10, "ALL SIX STAGES", 1, 1);
        assert_eq!(c.pixels().len(), 256);
    }

    #[test]
    fn tile_layer_camera_and_empty_tiles() {
        let t = Texture::from_rgba(2, 1, vec![255, 0, 0, 255, 0, 255, 0, 255]).unwrap();
        let mut c = Canvas::new(2, 1);
        c.clear(0x0000ff);
        c.tile_layer(&t, &[0, 1, 0], 3, 1, Camera { x: 1.0, y: 0.0 }, Some(0));
        assert_eq!(c.pixels(), &[0x00ff00, 0x0000ff]);
    }

    #[test]
    fn assembled_jeep_mirrors_parts_and_pixels_together() {
        let assets =
            crate::assets::Assets::load(&crate::assets::Assets::default_directory()).unwrap();
        let mut renderer = Renderer::new();
        renderer.sprite(
            &assets,
            0,
            3,
            128.0,
            119.0, // OAM Y is one pixel above the visible anchor.
            Camera::default(),
            None,
            false,
            false,
        );
        let normal = renderer.canvas.pixels().to_vec();
        assert!(normal.iter().any(|&pixel| pixel != 0));
        renderer.canvas.clear(0);
        renderer.sprite(
            &assets,
            0,
            3,
            128.0,
            119.0, // OAM Y is one pixel above the visible anchor.
            Camera::default(),
            None,
            true,
            true,
        );
        let mirrored = renderer.canvas.pixels();
        for y in 0..HEIGHT {
            for x in 0..WIDTH {
                assert_eq!(
                    normal[y * WIDTH + x],
                    mirrored[(HEIGHT - 1 - y) * WIDTH + WIDTH - 1 - x]
                );
            }
        }
    }

    #[test]
    fn projectile_rendering_preserves_native_hidden_frames_rockets_and_blast_waves() {
        use crate::{
            game::{Game, GameState},
            math::Vec2,
            player::WeaponLevel,
            terrain::CollisionField,
            weapon::{Projectile, ProjectileKind},
        };
        struct Ground;
        impl CollisionField for Ground {
            fn terrain_at(&self, _: Vec2) -> u8 {
                0
            }
        }
        let assets =
            crate::assets::Assets::load(&crate::assets::Assets::default_directory()).unwrap();
        let mut game = Game::new();
        game.start_stage(0, &assets);
        game.state = GameState::Playing;
        let position = Vec2::new(game.camera.x + 128., game.camera.y + 100.);
        let mut renderer = Renderer::new();
        let baseline = renderer.render(&game, &assets).to_vec();
        game.projectiles
            .push(Projectile::sentry_bullet(position, Vec2::default(), 64));
        assert_eq!(
            renderer.render(&game, &assets),
            baseline,
            "enemybullet is hidden before native delay"
        );
        for _ in 0..10 {
            game.projectiles[0].tick_enemy(position, &Ground);
        }
        let mut rocket = Projectile::player(
            position,
            Vec2::new(0., -4.),
            ProjectileKind::Rocket,
            0,
            22,
            WeaponLevel::FourWay,
        );
        rocket.tick_player(0);
        let mut wave = Projectile::blast_wave(position, 0, false);
        wave.tick_player(0);
        for (shot, frame, palette) in [
            (game.projectiles.remove(0), 16, 2),
            (rocket, 13, 1),
            (wave, 9, 3),
        ] {
            game.projectiles.clear();
            let mut expected = Renderer::new();
            expected.render(&game, &assets);
            expected.sprite(
                &assets,
                0,
                frame,
                position.x,
                position.y,
                game.camera,
                Some(palette),
                false,
                false,
            );
            game.projectiles.push(shot);
            assert_eq!(
                renderer.render(&game, &assets),
                expected.canvas.pixels(),
                "native projectile frame{frame}, palette{palette}"
            );
        }
    }

    #[test]
    fn native_actor_palette_preserves_metasprite_palette_bits() {
        // Original bank1:81B3..81B9 XORs flips, then ORs actor attributes.
        // Source52's tick8 has frame142/actorpalette0; source02 flashes
        // frame19 with actorpalette2. Their descriptor palettes are3 and1.
        let assets =
            crate::assets::Assets::load(&crate::assets::Assets::default_directory()).unwrap();
        let game = crate::game::Game::new();
        for (frame, actor_palette, tile) in [(142, 0, 169), (19, 2, 85)] {
            let mut expected = Renderer::new();
            let atlas = expected.world_textures(&assets, 0).1;
            let columns = atlas.width / 8;
            let index = tile + 3 * 256;
            expected.canvas.draw_region(
                atlas,
                Rect::new(
                    (index % columns * 8) as i32,
                    (index / columns * 16) as i32,
                    8,
                    16,
                ),
                Rect::new(76, 73, 8, 16),
                false,
                false,
            );
            for scene_path in [false, true] {
                let mut actual = Renderer::new();
                if scene_path {
                    actual.draw_scene(
                        &assets,
                        &game,
                        0,
                        vec![SceneSprite::new(
                            0,
                            frame,
                            80.,
                            80.,
                            Camera::default(),
                            Some(actor_palette),
                            false,
                            false,
                        )],
                    );
                } else {
                    actual.sprite(
                        &assets,
                        0,
                        frame,
                        80.,
                        80.,
                        Camera::default(),
                        Some(actor_palette),
                        false,
                        false,
                    );
                }
                let mut opaque = 0;
                for y in 73..89 {
                    for x in 76..84 {
                        let index = y * WIDTH + x;
                        assert_eq!(
                            actual.canvas.pixels()[index],
                            expected.canvas.pixels()[index],
                            "frame{frame} scene={scene_path} pixel({x},{y})"
                        );
                        opaque += usize::from(expected.canvas.pixels()[index] != 0);
                    }
                }
                assert!(opaque > 0);
            }
        }
    }

    #[test]
    fn every_stage_and_menu_renders_a_complete_frame() {
        use crate::game::{Game, GameState};
        let assets =
            crate::assets::Assets::load(&crate::assets::Assets::default_directory()).unwrap();
        let mut game = Game::new();
        let mut renderer = Renderer::new();
        for stage in 0..6 {
            game.start_stage(stage, &assets);
            let pixels = renderer.render(&game, &assets);
            assert_eq!(pixels.len(), WIDTH * HEIGHT);
            assert!(pixels.windows(2).any(|pair| pair[0] != pair[1]));
        }
        for state in [
            GameState::Title,
            GameState::Briefing { ticks: 10 },
            GameState::StageClear { ticks: 10 },
            GameState::GameOver { ticks: 10 },
            GameState::Continue { ticks: 10 },
            GameState::Ending { ticks: 10 },
        ] {
            game.state = state;
            assert_eq!(renderer.render(&game, &assets).len(), WIDTH * HEIGHT);
        }
    }

    #[test]
    fn invalid_texture_dimensions_are_rejected() {
        assert!(Texture::from_rgba(1, 1, vec![0; 3]).is_err());
        assert!(Texture::from_rgba(usize::MAX, 2, vec![]).is_err());
    }
}
