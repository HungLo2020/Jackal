//! Jeep movement, health, passengers and progression in world coordinates.
use crate::{input::PlayerInput, math::Vec2, terrain::CollisionField};

pub const STARTING_LIVES: u8 = 5;
pub const RESPAWN_PROTECTION: u16 = 160;
pub const DEATH_TICKS: u16 = 115;
pub const MOVE_PER_TICK: f32 = 1.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Facing {
    North,
    NorthEast,
    East,
    SouthEast,
    South,
    SouthWest,
    West,
    NorthWest,
}
impl Facing {
    pub fn vector(self) -> Vec2 {
        match self {
            Self::North => Vec2::new(0., -1.),
            Self::NorthEast => Vec2::new(1., -1.),
            Self::East => Vec2::new(1., 0.),
            Self::SouthEast => Vec2::new(1., 1.),
            Self::South => Vec2::new(0., 1.),
            Self::SouthWest => Vec2::new(-1., 1.),
            Self::West => Vec2::new(-1., 0.),
            Self::NorthWest => Vec2::new(-1., -1.),
        }
    }
    pub fn from_direction(x: i8, y: i8) -> Option<Self> {
        match (x, y) {
            (0, -1) => Some(Self::North),
            (1, -1) => Some(Self::NorthEast),
            (1, 0) => Some(Self::East),
            (1, 1) => Some(Self::SouthEast),
            (0, 1) => Some(Self::South),
            (-1, 1) => Some(Self::SouthWest),
            (-1, 0) => Some(Self::West),
            (-1, -1) => Some(Self::NorthWest),
            _ => None,
        }
    }
    pub fn turn_towards(self, target: Self) -> Self {
        const F: [Facing; 8] = [
            Facing::North,
            Facing::NorthEast,
            Facing::East,
            Facing::SouthEast,
            Facing::South,
            Facing::SouthWest,
            Facing::West,
            Facing::NorthWest,
        ];
        let a = self as i32;
        let b = target as i32;
        let delta = (b - a + 8) % 8;
        if delta == 0 {
            self
        } else {
            F[((a + if delta < 4 || (delta == 4 && b < a) {
                1
            } else {
                7
            }) % 8) as usize]
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum WeaponLevel {
    #[default]
    Grenade,
    Rocket,
    TwoWay,
    FourWay,
}
impl WeaponLevel {
    pub fn upgrade(&mut self) {
        *self = match self {
            Self::Grenade => Self::Rocket,
            Self::Rocket => Self::TwoWay,
            _ => Self::FourWay,
        };
    }
}

#[derive(Clone, Debug)]
pub struct Player {
    pub position: Vec2,
    pub facing: Facing,
    pub requested_facing: Facing,
    pub last_motion: Facing,
    pub river_current: bool,
    pub lives: u8,
    pub score: u32,
    pub weapon: WeaponLevel,
    pub passengers: u8,
    pub delivered: u32,
    pub invulnerable: u16,
    pub respawn_ticks: u16,
    pub enabled: bool,
    pub previous_input: PlayerInput,
    next_extra_life: u32,
}
impl Player {
    pub fn new(position: Vec2, enabled: bool) -> Self {
        Self {
            position,
            facing: Facing::North,
            requested_facing: Facing::North,
            last_motion: Facing::North,
            river_current: false,
            lives: if enabled { STARTING_LIVES } else { 0 },
            score: 0,
            weapon: WeaponLevel::Grenade,
            passengers: 0,
            delivered: 0,
            invulnerable: RESPAWN_PROTECTION,
            respawn_ticks: 0,
            enabled,
            previous_input: PlayerInput::default(),
            next_extra_life: 20_000,
        }
    }
    pub fn active(&self) -> bool {
        self.enabled && self.lives > 0 && self.respawn_ticks == 0
    }
    pub fn tick(
        &mut self,
        input: PlayerInput,
        level: &impl CollisionField,
        stage: usize,
        tick: u64,
    ) {
        self.invulnerable = self.invulnerable.saturating_sub(1);
        if self.respawn_ticks > 0 {
            self.respawn_ticks -= 1;
            if self.respawn_ticks == 1 {
                self.lives = self.lives.saturating_sub(1);
            }
            if self.respawn_ticks == 0 {
                self.invulnerable = RESPAWN_PROTECTION;
            }
            return;
        }
        if !self.active() {
            return;
        }
        // Impossible opposite D-pad combinations select the original idle path.
        let (x, y) = if (input.up && input.down) || (input.left && input.right) {
            (0, 0)
        } else {
            input.direction()
        };
        if let Some(facing) = Facing::from_direction(x, y) {
            self.requested_facing = facing;
            self.last_motion = facing;
            if tick.is_multiple_of(4) {
                self.facing = self.facing.turn_towards(facing);
            }
        } else {
            self.requested_facing = self.facing;
            if self.river_current && tick.is_multiple_of(2) {
                self.position.y += 1.;
            }
            return;
        }
        let speed = if stage == 3 && level.terrain_at(self.position) == 3 {
            MOVE_PER_TICK * 0.5
        } else {
            MOVE_PER_TICK
        };
        // The original intentionally moves one pixel on each diagonal axis.
        let dx = x as f32 * speed;
        let dy = y as f32 * speed;
        let probes = match self.requested_facing {
            Facing::North => [Vec2::new(-4., -8.), Vec2::new(4., -8.)],
            Facing::NorthEast => [Vec2::new(0., -6.), Vec2::new(6., 0.)],
            Facing::East => [Vec2::new(8., -4.), Vec2::new(8., 4.)],
            Facing::SouthEast => [Vec2::new(6., 0.), Vec2::new(0., 6.)],
            Facing::South => [Vec2::new(-4., 8.), Vec2::new(4., 8.)],
            Facing::SouthWest => [Vec2::new(-6., 0.), Vec2::new(0., 6.)],
            Facing::West => [Vec2::new(-8., -4.), Vec2::new(-8., 4.)],
            Facing::NorthWest => [Vec2::new(-6., 0.), Vec2::new(0., -6.)],
        };
        // The original collision lookup adds the proposed velocity before these
        // two leading-edge offsets. A full bounding box blocks valid turns.
        let clear = probes.into_iter().all(|offset| {
            let terrain = level.terrain_at(self.position + Vec2::new(dx, dy) + offset);
            terrain == 0 || (stage == 3 && terrain == 3)
        });
        if self.river_current {
            self.position += Vec2::new(
                if clear { dx } else { 0. },
                if dy > 0. {
                    dy
                } else if dy < 0. {
                    -0.5
                } else {
                    0.5
                },
            );
        } else if clear {
            self.position += Vec2::new(dx, dy);
        }
    }
    pub fn hit(&mut self) -> bool {
        if !self.active() || self.invulnerable > 0 {
            return false;
        }
        self.weapon = WeaponLevel::Grenade;
        self.passengers = 0;
        self.respawn_ticks = DEATH_TICKS;
        self.invulnerable = DEATH_TICKS + RESPAWN_PROTECTION;
        true
    }
    pub fn add_score(&mut self, amount: u32) {
        self.score = self.score.saturating_add(amount);
        while self.score >= self.next_extra_life {
            self.lives = self.lives.saturating_add(1).min(9);
            self.next_extra_life = self.next_extra_life.saturating_add(50_000);
            if self.next_extra_life == u32::MAX {
                break;
            }
        }
    }
    pub fn rescue(&mut self, special: bool) -> bool {
        if !self.active() || self.passengers == u8::MAX {
            return false;
        }
        self.passengers += 1;
        if special {
            self.weapon.upgrade();
        }
        true
    }
    pub fn deliver_one(&mut self) -> bool {
        if !self.active() || self.passengers == 0 {
            return false;
        }
        self.passengers -= 1;
        self.delivered += 1;
        if [4, 9, 15].contains(&self.delivered) {
            self.weapon.upgrade();
        }
        true
    }
    pub fn animation(&self) -> (&'static str, bool, bool) {
        match self.facing {
            Facing::North => ("jeep_north", false, false),
            Facing::NorthEast => ("jeep_northeast", false, false),
            Facing::East => ("jeep_east", false, false),
            Facing::SouthEast => ("jeep_northeast", false, true),
            Facing::South => ("jeep_north", false, true),
            Facing::SouthWest => ("jeep_northeast", true, true),
            Facing::West => ("jeep_east", true, false),
            Facing::NorthWest => ("jeep_northeast", true, false),
        }
    }
}

#[cfg(test)]
mod movement_tests {
    use super::*;
    struct ProbeField {
        blocked: Vec2,
    }
    impl CollisionField for ProbeField {
        fn terrain_at(&self, point: Vec2) -> u8 {
            u8::from(point == self.blocked)
        }
    }
    #[test]
    fn leading_edge_blocks_movement_without_testing_trailing_corners() {
        let mut player = Player::new(Vec2::new(32., 32.), true);
        let right = PlayerInput {
            right: true,
            ..Default::default()
        };
        player.tick(
            right,
            &ProbeField {
                blocked: Vec2::new(41., 28.),
            },
            0,
            0,
        );
        assert_eq!(player.position, Vec2::new(32., 32.));
        player.tick(
            right,
            &ProbeField {
                blocked: Vec2::new(29., 26.),
            },
            0,
            1,
        );
        assert_eq!(player.position, Vec2::new(33., 32.));
    }
    #[test]
    fn releasing_direction_stops_rotation_and_aims_along_visible_jeep() {
        let mut player = Player::new(Vec2::new(32., 32.), true);
        let field = ProbeField {
            blocked: Vec2::new(-1., -1.),
        };
        player.tick(
            PlayerInput {
                right: true,
                ..Default::default()
            },
            &field,
            0,
            0,
        );
        assert_eq!(player.facing, Facing::NorthEast);
        player.tick(PlayerInput::default(), &field, 0, 4);
        assert_eq!(player.requested_facing, Facing::NorthEast);
        assert_eq!(player.facing, Facing::NorthEast);
    }
    #[test]
    fn half_turn_ties_match_original_direction_order() {
        assert_eq!(Facing::North.turn_towards(Facing::South), Facing::NorthWest);
        assert_eq!(Facing::South.turn_towards(Facing::North), Facing::SouthWest);
    }
    #[test]
    fn opposed_axis_cancels_the_whole_direction() {
        let mut player = Player::new(Vec2::new(32., 32.), true);
        player.tick(
            PlayerInput {
                up: true,
                down: true,
                right: true,
                ..Default::default()
            },
            &ProbeField {
                blocked: Vec2::new(-1., -1.),
            },
            0,
            0,
        );
        assert_eq!(player.position, Vec2::new(32., 32.));
    }
}
