use serde::{Deserialize, Serialize};
use std::ops::{Add, AddAssign, Mul, Sub};

#[derive(Clone, Copy, Debug, Default, PartialEq, Deserialize, Serialize)]
pub struct Vec2 {
    pub x: f32,
    pub y: f32,
}
impl Vec2 {
    pub const ZERO: Self = Self::new(0.0, 0.0);
    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }
    pub fn length(self) -> f32 {
        self.x.hypot(self.y)
    }
    pub fn normalized(self) -> Self {
        let len = self.length();
        if len > 0.0 {
            self * (1.0 / len)
        } else {
            Self::ZERO
        }
    }
    pub fn distance(self, other: Self) -> f32 {
        (self - other).length()
    }
}
impl Add for Vec2 {
    type Output = Self;
    fn add(self, rhs: Self) -> Self {
        Self::new(self.x + rhs.x, self.y + rhs.y)
    }
}
impl AddAssign for Vec2 {
    fn add_assign(&mut self, rhs: Self) {
        *self = *self + rhs;
    }
}
impl Sub for Vec2 {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self {
        Self::new(self.x - rhs.x, self.y - rhs.y)
    }
}
impl Mul<f32> for Vec2 {
    type Output = Self;
    fn mul(self, rhs: f32) -> Self {
        Self::new(self.x * rhs, self.y * rhs)
    }
}
impl From<[f32; 2]> for Vec2 {
    fn from(v: [f32; 2]) -> Self {
        Self::new(v[0], v[1])
    }
}
impl From<Vec2> for [f32; 2] {
    fn from(v: Vec2) -> Self {
        [v.x, v.y]
    }
}

/// Swept segment test prevents fast bullets passing through small targets.
pub fn segment_hits_circle(from: Vec2, to: Vec2, center: Vec2, radius: f32) -> bool {
    let delta = to - from;
    let len2 = delta.x * delta.x + delta.y * delta.y;
    let t = if len2 == 0.0 {
        0.0
    } else {
        (((center - from).x * delta.x + (center - from).y * delta.y) / len2).clamp(0.0, 1.0)
    };
    (from + delta * t).distance(center) <= radius
}
