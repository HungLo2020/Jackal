impl Video {
    pub fn water_signature(&self) -> u64 {
        let mut hash = 0xcbf29ce484222325u64;
        for tile in 222..234 {
            for y in 0..8 {
                for x in 0..8 {
                    hash ^=
                        self.tile_pixel(((self.ctrl as usize & 16) << 8) + tile * 16, x, y) as u64;
                    hash = hash.wrapping_mul(0x100000001b3);
                }
            }
        }
        hash
    }
    pub fn background_colors(&self) -> Vec<u32> {
        (0..16).map(|i| self.color(0x3f00 + i, 0)).collect()
    }
}
