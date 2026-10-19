static SOUNDS: std::sync::Mutex<Vec<u8>> = std::sync::Mutex::new(Vec::new());
pub fn record_sound(command: u8) {
    SOUNDS.lock().unwrap().push(command);
}
fn drain_sounds() -> Vec<u8> {
    std::mem::take(&mut *SOUNDS.lock().unwrap())
}
use std::collections::HashMap;
struct Capture {
    tiles: Vec<Vec<u32>>,
    tile_ids: HashMap<Vec<u32>, u32>,
    frames: Vec<serde_json::Value>,
    last: Vec<u32>,
    duration: u32,
    ticks: u32,
    sounds: Vec<[u32; 2]>,
}
impl Capture {
    fn new() -> Self {
        Self {
            tiles: Vec::new(),
            tile_ids: HashMap::new(),
            frames: Vec::new(),
            last: Vec::new(),
            duration: 0,
            ticks: 0,
            sounds: Vec::new(),
        }
    }
    fn sound_commands(&mut self, commands: Vec<u8>) {
        for command in commands {
            self.sounds.push([self.ticks, command as u32]);
        }
    }
    fn frame(&mut self, pixels: &[u32]) {
        self.ticks += 1;
        let mut ids = Vec::with_capacity(960);
        for ty in 0..30 {
            for tx in 0..32 {
                let mut tile = Vec::with_capacity(64);
                for y in 0..8 {
                    tile.extend_from_slice(
                        &pixels[(ty * 8 + y) * 256 + tx * 8..(ty * 8 + y) * 256 + tx * 8 + 8],
                    );
                }
                let id = if let Some(id) = self.tile_ids.get(&tile) {
                    *id
                } else {
                    let id = self.tiles.len() as u32;
                    self.tiles.push(tile.clone());
                    self.tile_ids.insert(tile, id);
                    id
                };
                ids.push(id);
            }
        }
        if ids == self.last {
            self.duration += 1;
        } else {
            self.flush();
            self.last = ids;
            self.duration = 1;
        }
    }
    fn flush(&mut self) {
        if self.duration == 0 {
            return;
        }
        let mut runs: Vec<[u32; 2]> = Vec::new();
        for &id in &self.last {
            if let Some(last) = runs.last_mut() {
                if last[1] == id {
                    last[0] += 1;
                    continue;
                }
            }
            runs.push([1, id]);
        }
        self.frames
            .push(serde_json::json!({"duration_ticks":self.duration,"tiles":runs}));
    }
    fn save(mut self, root: &str, name: &str, source_start_frame: u32) {
        self.flush();
        let dir = format!("{root}/assets/cinematics/{name}");
        std::fs::create_dir_all(&dir).unwrap();
        let columns = 64;
        let rows = self.tiles.len().div_ceil(columns);
        let mut img = image::RgbaImage::new((columns * 8) as u32, (rows * 8) as u32);
        for (i, tile) in self.tiles.iter().enumerate() {
            for (p, &c) in tile.iter().enumerate() {
                img.put_pixel(
                    ((i % columns) * 8 + p % 8) as u32,
                    ((i / columns) * 8 + p / 8) as u32,
                    image::Rgba([(c >> 16) as u8, (c >> 8) as u8, c as u8, 255]),
                );
            }
        }
        img.save(format!("{dir}/tiles.png")).unwrap();
        let duration: u64 = self
            .frames
            .iter()
            .map(|f| f["duration_ticks"].as_u64().unwrap())
            .sum();
        let metadata = serde_json::json!({"source":"historic reference e8fc3391d5b27fcd98b82fd27984141840449708", "source_start_frame":source_start_frame,"atlas":"tiles.png","tile_size":8,"atlas_columns":columns,"duration_ticks":duration,"sound_commands":self.sounds,"frames":self.frames});
        std::fs::write(
            format!("{dir}/timeline.json"),
            serde_json::to_string(&metadata).unwrap(),
        )
        .unwrap();
        println!(
            "{name}: {duration} ticks, {} unique tiles, {} frames",
            self.tiles.len(),
            metadata["frames"].as_array().unwrap().len()
        );
    }
}
