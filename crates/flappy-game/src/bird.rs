//! The player bird: vertical physics, flap impulse, and a cosmetic tilt.
//!
//! Faithful to the original `bird.py`: a flap sets the velocity to
//! [`JUMP_VELOCITY`], and each step adds [`GRAVITY`] after integrating
//! position. The tilt is purely visual.

use crate::geometry::Aabb;

/// Upward impulse applied by a flap (negative because y grows downward).
pub const JUMP_VELOCITY: f32 = -20.0;
/// Per-step gravitational acceleration.
pub const GRAVITY: f32 = 2.0;

/// Collision/sprite width of the bird, in pixels.
pub const WIDTH: f32 = 40.0;
/// Collision/sprite height of the bird, in pixels.
pub const HEIGHT: f32 = 30.0;

const MAX_ROTATION: f32 = 25.0;
const ROTATION_VELOCITY: f32 = 20.0;

/// The bird, positioned by its top-left corner like the original sprite.
#[derive(Clone, Copy, Debug)]
pub struct Bird {
    /// Top-left x (constant during play; the world scrolls past it).
    pub x: f32,
    /// Top-left y.
    pub y: f32,
    /// Vertical velocity.
    pub vel: f32,
    /// Cosmetic tilt in degrees (positive = nose up).
    pub tilt: f32,
    /// The y at which the last flap occurred — drives the tilt logic.
    height: f32,
}

impl Bird {
    /// Spawn a bird at the given top-left position.
    pub fn new(x: f32, y: f32) -> Self {
        Self {
            x,
            y,
            vel: 0.0,
            tilt: 0.0,
            height: y,
        }
    }

    /// Apply a flap: snap the velocity to the jump impulse.
    pub fn jump(&mut self) {
        self.vel = JUMP_VELOCITY;
        self.height = self.y;
    }

    /// Advance the bird one physics step.
    pub fn update(&mut self) {
        self.y += self.vel;
        self.vel += GRAVITY;

        // Tilt: nose up briefly after a flap, then rotate toward a nose-dive.
        if self.y < self.height + 50.0 {
            if self.tilt < MAX_ROTATION {
                self.tilt = MAX_ROTATION;
            }
        } else if self.tilt > -90.0 {
            self.tilt -= ROTATION_VELOCITY;
        }
    }

    /// Centre of the bird as `(x, y, vy)` — the controller's view of the state.
    ///
    /// Mirrors the original `physical_position()`.
    pub fn physical_position(&self) -> (f32, f32, f32) {
        (self.x + WIDTH / 2.0, self.y + HEIGHT / 2.0, self.vel)
    }

    /// Collision box of the bird.
    pub fn bounds(&self) -> Aabb {
        Aabb::new(self.x, self.y, WIDTH, HEIGHT)
    }
}
