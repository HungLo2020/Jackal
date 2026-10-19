impl Video {
    pub fn verify_background_mask(&self) -> bool {
        for pal in 0..4 {
            for tile in 0..256 {
                for y in 0..8 {
                    for x in 0..8 {
                        let v = self.tile_pixel(((self.ctrl as usize & 16) << 8) + tile * 16, x, y);
                        let color =
                            self.color(if v == 0 { 0x3f00 } else { 0x3f00 + pal * 4 + v }, 0);
                        if (v == 0) != (color == self.color(0x3f00, 0)) {
                            return false;
                        }
                    }
                }
            }
        }
        true
    }
}
