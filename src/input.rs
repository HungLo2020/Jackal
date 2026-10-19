#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PlayerInput {
    pub up: bool,
    pub down: bool,
    pub left: bool,
    pub right: bool,
    pub gun: bool,
    pub grenade: bool,
}
impl PlayerInput {
    pub fn direction(self) -> (i8, i8) {
        (
            (self.right as i8) - (self.left as i8),
            (self.down as i8) - (self.up as i8),
        )
    }
    /// Input recordings use the original controller button order, not runtime registers.
    pub fn from_buttons(buttons: u8) -> Self {
        Self {
            grenade: buttons & 1 != 0,
            gun: buttons & 2 != 0,
            up: buttons & 16 != 0,
            down: buttons & 32 != 0,
            left: buttons & 64 != 0,
            right: buttons & 128 != 0,
        }
    }
}
#[derive(Clone, Copy, Debug, Default)]
pub struct Input {
    pub players: [PlayerInput; 2],
    pub start: bool,
    pub select: bool,
    pub pause: bool,
}
