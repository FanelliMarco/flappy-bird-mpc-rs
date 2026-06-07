//! A pipe pair with a vertical gap the bird must fly through.
//!
//! Faithful to the original `pipe.py`: a random gap-top in `[50, 450)`, a fixed
//! [`GAP`] below it, and constant leftward scroll at [`VELOCITY`] px/step.

use crate::bird::Bird;
use crate::geometry::Aabb;
use rand::Rng;

/// Vertical size of the gap between the top and bottom pipe.
pub const GAP: f32 = 200.0;
/// Leftward scroll speed in pixels per step.
pub const VELOCITY: f32 = 5.0;
/// Width of a pipe in pixels.
pub const WIDTH: f32 = 80.0;

/// Y of the base/ground; the bottom pipe is drawn down to here.
pub const GROUND_Y: f32 = 730.0;

/// A scrolling pipe pair.
#[derive(Clone, Copy, Debug)]
pub struct Pipe {
    /// Left edge x of both pipes.
    pub x: f32,
    /// Top edge of the gap (bottom of the upper pipe). The original `height`.
    pub gap_top: f32,
    /// Bottom edge of the gap (top of the lower pipe). The original `bottom`.
    pub gap_bottom: f32,
    /// Whether the bird has already cleared this pipe (for scoring).
    pub passed: bool,
}

impl Pipe {
    /// Create a pipe at horizontal position `x` with a randomly placed gap.
    pub fn new(x: f32) -> Self {
        let mut rng = rand::thread_rng();
        Self::with_gap_top(x, rng.gen_range(50.0..450.0))
    }

    /// Create a pipe with an explicit gap-top (handy for tests / determinism).
    pub fn with_gap_top(x: f32, gap_top: f32) -> Self {
        Self {
            x,
            gap_top,
            gap_bottom: gap_top + GAP,
            passed: false,
        }
    }

    /// Scroll the pipe one step to the left.
    pub fn update(&mut self) {
        self.x -= VELOCITY;
    }

    /// Right edge of the pipe (`x + WIDTH`).
    pub fn right(&self) -> f32 {
        self.x + WIDTH
    }

    /// Collision box of the upper pipe (from the top of the screen to the gap).
    pub fn top_bounds(&self) -> Aabb {
        Aabb::new(self.x, 0.0, WIDTH, self.gap_top)
    }

    /// Collision box of the lower pipe (from the gap down to the ground).
    pub fn bottom_bounds(&self) -> Aabb {
        Aabb::new(self.x, self.gap_bottom, WIDTH, GROUND_Y - self.gap_bottom)
    }

    /// Whether the bird collides with either pipe of this pair.
    pub fn collides(&self, bird: &Bird) -> bool {
        let b = bird.bounds();
        b.intersects(&self.top_bounds()) || b.intersects(&self.bottom_bounds())
    }

    /// Gap edges `(lower, upper)` for the MPC controller, where `lower` is the
    /// larger-y bottom edge — matching the original `limits = [bottom, top]`.
    pub fn controller_limits(&self) -> (f32, f32) {
        (self.gap_bottom, self.gap_top)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bird::Bird;

    #[test]
    fn bird_in_gap_does_not_collide() {
        let pipe = Pipe::with_gap_top(200.0, 250.0); // gap 250..450
        let mut bird = Bird::new(210.0, 340.0); // centred in the gap, overlapping x
        bird.vel = 0.0;
        assert!(!pipe.collides(&bird));
    }

    #[test]
    fn bird_into_top_pipe_collides() {
        let pipe = Pipe::with_gap_top(200.0, 250.0);
        let bird = Bird::new(210.0, 100.0); // up in the top pipe
        assert!(pipe.collides(&bird));
    }

    #[test]
    fn limits_are_bottom_then_top() {
        let pipe = Pipe::with_gap_top(200.0, 250.0);
        let (lower, upper) = pipe.controller_limits();
        assert_eq!((lower, upper), (450.0, 250.0));
        assert!(lower > upper, "screen-y grows downward");
    }
}
