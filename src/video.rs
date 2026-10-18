//! Software tile and sprite compositor for Jackal's writable CHR and vertical
//! nametable wiring. Register latches are modeled; rendering is frame based.
pub const WIDTH: usize = 256;
pub const HEIGHT: usize = 240;

const COLORS: [u32; 64] = [
    0x626262, 0x001fb2, 0x2404c8, 0x5200b2, 0x730076, 0x800024, 0x730b00, 0x522800, 0x244400,
    0x005700, 0x005c00, 0x005324, 0x003c76, 0, 0, 0, 0xababab, 0x0d57ff, 0x4b30ff, 0x8a13ff,
    0xbc08d6, 0xd21269, 0xc72e00, 0x9d5400, 0x607b00, 0x209800, 0x00a300, 0x009942, 0x007db4, 0, 0,
    0, 0xffffff, 0x53aeff, 0x9085ff, 0xd365ff, 0xff57ff, 0xff5dcf, 0xff7757, 0xfa9e00, 0xbdc700,
    0x7ae700, 0x43f611, 0x26ef7e, 0x2cd5f6, 0x4e4e4e, 0, 0, 0xffffff, 0xb6e1ff, 0xced1ff, 0xe9c3ff,
    0xffbcff, 0xffbdf4, 0xffc6c3, 0xffd59a, 0xe9e681, 0xcef481, 0xb6fb9a, 0xa9fac3, 0xa9f0f4,
    0xb8b8b8, 0, 0,
];

#[derive(Clone, Copy)]
struct SplitTop {
    row: usize,
    t: u16,
    fine_x: u8,
    ctrl: u8,
    mask: u8,
}

pub struct Video {
    pub ctrl: u8,
    pub mask: u8,
    pub status: u8,
    pub oam: [u8; 256],
    memory: Box<[u8; 0x4000]>,
    oam_addr: u8,
    v: u16,
    t: u16,
    fine_x: u8,
    second_write: bool,
    read_buffer: u8,
    open_bus: u8,
    pixels: Vec<u32>,
    split_top: Option<SplitTop>,
}

impl Default for Video {
    fn default() -> Self {
        Self::new()
    }
}

impl Video {
    pub fn new() -> Self {
        Self {
            ctrl: 0,
            mask: 0,
            status: 0,
            oam: [255; 256],
            memory: Box::new([0; 0x4000]),
            oam_addr: 0,
            v: 0,
            t: 0,
            fine_x: 0,
            second_write: false,
            read_buffer: 0,
            open_bus: 0,
            pixels: vec![0; WIDTH * HEIGHT],
            split_top: None,
        }
    }

    pub fn set_vblank(&mut self, active: bool) {
        if active {
            self.status |= 0x80;
        } else {
            self.status &= !0x80;
        }
    }

    pub fn begin_frame(&mut self) {
        self.status &= !0xe0;
        self.split_top = None;
    }

    /// Preserve registers for rows before `row`, called before Jackal's
    /// transition delay changes horizontal scroll. The split changes horizontal
    /// scroll only: the live vertical counter continues from the top setup.
    pub fn capture_split_top(&mut self, row: usize) {
        self.split_top = Some(SplitTop {
            row: row.min(HEIGHT),
            t: self.t,
            fine_x: self.fine_x,
            ctrl: self.ctrl,
            mask: self.mask,
        });
    }

    fn row_registers(&self, y: usize) -> (u8, u8, u8) {
        match self.split_top {
            Some(top) if y < top.row => (top.ctrl, top.mask, top.fine_x),
            _ => (self.ctrl, self.mask, self.fine_x),
        }
    }

    fn index(address: usize) -> usize {
        let a = address & 0x3fff;
        if a >= 0x3f00 {
            let p = a & 31;
            0x3f00 + if p & 0x13 == 0x10 { p & 15 } else { p }
        } else if a >= 0x2000 {
            0x2000 + ((a - 0x2000) & 0x7ff)
        } else {
            a
        }
    }

    fn read(&self, address: usize) -> u8 {
        self.memory[Self::index(address)]
    }

    fn increment(&mut self) {
        self.v = self.v.wrapping_add(if self.ctrl & 4 == 0 { 1 } else { 32 }) & 0x7fff;
    }

    pub fn read_register(&mut self, address: u16) -> u8 {
        let value = match address & 7 {
            2 => {
                let value = (self.status & 0xe0) | (self.open_bus & 31);
                self.status &= !0x80;
                self.second_write = false;
                value
            }
            4 => self.oam[self.oam_addr as usize],
            7 => {
                let a = (self.v & 0x3fff) as usize;
                let data = self.read(a);
                let value = if a >= 0x3f00 {
                    (data & if self.mask & 1 != 0 { 0x30 } else { 0x3f }) | (self.open_bus & 0xc0)
                } else {
                    self.read_buffer
                };
                self.read_buffer = self.read(if a >= 0x3f00 { a - 0x1000 } else { a });
                self.increment();
                value
            }
            _ => self.open_bus,
        };
        self.open_bus = value;
        value
    }

    pub fn write_register(&mut self, address: u16, value: u8) {
        self.open_bus = value;
        match address & 7 {
            0 => {
                self.ctrl = value;
                self.t = (self.t & !0x0c00) | ((value as u16 & 3) << 10);
            }
            1 => self.mask = value,
            3 => self.oam_addr = value,
            4 => {
                self.oam[self.oam_addr as usize] = value;
                self.oam_addr = self.oam_addr.wrapping_add(1);
            }
            5 => {
                if self.second_write {
                    self.t = (self.t & !0x73e0)
                        | ((value as u16 & 7) << 12)
                        | ((value as u16 & 0xf8) << 2);
                } else {
                    self.fine_x = value & 7;
                    self.t = (self.t & !31) | (value as u16 >> 3);
                }
                self.second_write = !self.second_write;
            }
            6 => {
                if self.second_write {
                    self.t = (self.t & 0x7f00) | value as u16;
                    self.v = self.t;
                } else {
                    self.t = (self.t & 0xff) | ((value as u16 & 0x3f) << 8);
                }
                self.second_write = !self.second_write;
            }
            7 => {
                let index = Self::index(self.v as usize);
                self.memory[index] = if index >= 0x3f00 { value & 0x3f } else { value };
                self.increment();
            }
            _ => {}
        }
    }

    pub fn dma(&mut self, bytes: &[u8]) {
        for (i, &byte) in bytes.iter().take(256).enumerate() {
            self.oam[(self.oam_addr as usize + i) & 255] = byte;
        }
    }

    fn color(&self, address: usize, mask: u8) -> u32 {
        let index = self.read(address) & if mask & 1 != 0 { 0x30 } else { 0x3f };
        let rgb = COLORS[index as usize];
        // Approximate the analog emphasis attenuation in RGB space.
        let mut r = (rgb >> 16) & 255;
        let mut g = (rgb >> 8) & 255;
        let mut b = rgb & 255;
        if mask & 0x20 != 0 {
            g = g * 3 / 4;
            b = b * 3 / 4;
        }
        if mask & 0x40 != 0 {
            r = r * 3 / 4;
            b = b * 3 / 4;
        }
        if mask & 0x80 != 0 {
            r = r * 3 / 4;
            g = g * 3 / 4;
        }
        (r << 16) | (g << 8) | b
    }

    fn tile_pixel(&self, address: usize, x: usize, y: usize) -> usize {
        let bit = 7 - x;
        (((self.read(address + y) >> bit) & 1) | (((self.read(address + y + 8) >> bit) & 1) << 1))
            as usize
    }

    /// Render a frame using the most recent scroll setup. The host schedules
    /// vblank/NMI separately. Mid-frame register changes require host snapshots.
    pub fn render(&mut self) -> &[u32] {
        let mut opaque = vec![false; WIDTH * HEIGHT];
        let mut row = self.split_top.map_or(self.t, |top| top.t);
        for y in 0..HEIGHT {
            if self.split_top.is_some_and(|top| y == top.row) {
                row = (row & !0x041f) | (self.t & 0x041f);
            }
            let (ctrl, mask, fine_x) = self.row_registers(y);
            let bank = (ctrl as usize & 0x10) << 8;
            let backdrop = self.color(0x3f00, mask);
            self.pixels[y * WIDTH..(y + 1) * WIDTH].fill(backdrop);
            let coarse_y = (row as usize >> 5) & 31;
            let fine_y = (row as usize >> 12) & 7;
            for x in 0..WIDTH {
                if mask & 8 == 0 || (x < 8 && mask & 2 == 0) {
                    continue;
                }
                let sx = (row as usize & 31) * 8 + fine_x as usize + x;
                let tx = (sx / 8) & 31;
                let nt = 0x2000 + (((row as usize & 0xc00) ^ ((sx / 256) * 0x400)) & 0xc00);
                let tile = self.read(nt + coarse_y * 32 + tx) as usize;
                let color = self.tile_pixel(bank + tile * 16, sx & 7, fine_y);
                if color == 0 {
                    continue;
                }
                let attr = self.read(nt + 0x3c0 + (coarse_y / 4) * 8 + tx / 4);
                let shift = ((coarse_y & 2) << 1) | (tx & 2);
                let palette = ((attr >> shift) & 3) as usize;
                self.pixels[y * WIDTH + x] = self.color(0x3f00 + palette * 4 + color, mask);
                opaque[y * WIDTH + x] = true;
            }
            if row & 0x7000 != 0x7000 {
                row += 0x1000;
            } else {
                row &= !0x7000;
                let cy = (row >> 5) & 31;
                let next = if cy == 29 {
                    row ^= 0x800;
                    0
                } else {
                    (cy + 1) & 31
                };
                row = (row & !0x3e0) | (next << 5);
            }
        }
        {
            for y in 0..HEIGHT {
                let (ctrl, mask, _) = self.row_registers(y);
                if mask & 0x10 == 0 {
                    continue;
                }
                let height = if ctrl & 0x20 != 0 { 16 } else { 8 };
                let mut count = 0;
                let mut occupied = [false; WIDTH];
                for s in 0..64 {
                    let i = s * 4;
                    let top = self.oam[i] as usize + 1;
                    if y < top || y >= top + height {
                        continue;
                    }
                    count += 1;
                    if count > 8 {
                        self.status |= 0x20;
                        break;
                    }
                    let tile = self.oam[i + 1] as usize;
                    let attr = self.oam[i + 2];
                    let left = self.oam[i + 3] as usize;
                    let dy = if attr & 0x80 != 0 {
                        height - 1 - (y - top)
                    } else {
                        y - top
                    };
                    let address = if height == 16 {
                        (tile & 1) * 0x1000 + (tile & !1) * 16 + (dy / 8) * 16
                    } else {
                        ((ctrl as usize & 8) << 9) + tile * 16
                    };
                    for dx in 0..8 {
                        let x = left + dx;
                        if x >= WIDTH || (x < 8 && mask & 4 == 0) {
                            continue;
                        }
                        let tx = if attr & 0x40 != 0 { 7 - dx } else { dx };
                        let color = self.tile_pixel(address, tx, dy & 7);
                        if color == 0 {
                            continue;
                        }
                        let p = y * WIDTH + x;
                        if s == 0 && opaque[p] && x != 255 {
                            self.status |= 0x40;
                        }
                        if occupied[x] {
                            continue;
                        }
                        occupied[x] = true;
                        if attr & 0x20 == 0 || !opaque[p] {
                            self.pixels[p] =
                                self.color(0x3f10 + (attr as usize & 3) * 4 + color, mask);
                        }
                    }
                }
            }
        }
        &self.pixels
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn put(v: &mut Video, address: u16, bytes: &[u8]) {
        v.read_register(0x2002);
        v.write_register(0x2006, (address >> 8) as u8);
        v.write_register(0x2006, address as u8);
        for &b in bytes {
            v.write_register(0x2007, b);
        }
    }
    fn scroll(v: &mut Video, x: u8, y: u8) {
        v.read_register(0x2002);
        v.write_register(0x2000, v.ctrl & !3);
        v.write_register(0x2005, x);
        v.write_register(0x2005, y);
    }
    #[test]
    fn mirrors_palette_buffer_and_increment() {
        let mut v = Video::new();
        put(&mut v, 0x2000, &[17, 18]);
        assert_eq!(v.read(0x2800), 17);
        assert_eq!(v.read(0x3001), 18);
        put(&mut v, 0x3f10, &[33]);
        assert_eq!(v.read(0x3f00), 33);
        assert_eq!(v.read(0x3f30), 33);
        put(&mut v, 0x2000, &[]);
        assert_eq!(v.read_register(0x2007), 0);
        assert_eq!(v.read_register(0x2007), 17);
        assert_eq!(v.read_register(0x2007), 18);
        v.write_register(0x2000, 4);
        put(&mut v, 0x2400, &[4, 5]);
        assert_eq!(v.read(0x2420), 5);
    }
    #[test]
    fn scrolling_sprite_priority_flips_and_hit() {
        let mut v = Video::new();
        v.mask = 0x1e;
        put(&mut v, 16, &[0x80]);
        put(&mut v, 0x2020, &[1]);
        put(&mut v, 0x3f00, &[15, 1]);
        put(&mut v, 0x3f11, &[33]);
        scroll(&mut v, 0, 0);
        v.oam[..4].copy_from_slice(&[7, 1, 0x20, 0]);
        assert_eq!(v.render()[8 * WIDTH], COLORS[1]);
        assert_ne!(v.status & 0x40, 0);
        v.oam[2] = 0;
        assert_eq!(v.render()[8 * WIDTH], COLORS[33]);
        v.oam[2] = 0xc0;
        assert_eq!(v.render()[15 * WIDTH + 7], COLORS[33]);
        v.oam.fill(255);
        scroll(&mut v, 1, 0);
        assert_eq!(v.render()[8 * WIDTH], COLORS[15]);
    }
    #[test]
    fn split_scroll_preserves_top_and_continues_vertical_counter() {
        let mut v = Video::new();
        v.mask = 0x0a;
        put(&mut v, 16, &[0xff; 8]);
        put(&mut v, 0x2000, &[1]);
        put(&mut v, 0x2020, &[1]);
        put(&mut v, 0x3f00, &[15, 1]);
        scroll(&mut v, 0, 0);
        v.capture_split_top(8);
        // The bottom scroll's vertical field must not reset the live counter.
        scroll(&mut v, 4, 120);
        let pixels = v.render();
        assert_eq!(pixels[7], COLORS[1]);
        assert_eq!(pixels[8 * WIDTH + 3], COLORS[1]);
        assert_eq!(pixels[8 * WIDTH + 4], COLORS[15]);
        v.begin_frame();
        assert!(v.split_top.is_none());
    }
    #[test]
    fn dma_wraps_and_tall_sprites_select_pattern_bank() {
        let mut v = Video::new();
        v.write_register(0x2003, 255);
        v.dma(&[2, 3]);
        assert_eq!(v.oam[255], 2);
        assert_eq!(v.oam[0], 3);
        v.oam.fill(255);
        put(&mut v, 0x1010, &[0x80]);
        put(&mut v, 0x3f11, &[33]);
        v.ctrl = 0x20;
        v.mask = 0x14;
        v.oam[..4].copy_from_slice(&[0, 1, 0, 0]);
        assert_eq!(v.render()[9 * WIDTH], COLORS[33]);
    }
}
