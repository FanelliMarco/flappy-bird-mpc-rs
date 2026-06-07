//! The scrolling ground strip at the bottom of the screen.
//!
//! Faithful to the original `base.py`: two tiles slide left at [`VELOCITY`] and
//! wrap around to create an endless floor.

/// Leftward scroll speed in pixels per step.
pub const VELOCITY: f32 = 5.0;
/// Width of a single ground tile in pixels.
pub const WIDTH: f32 = 700.0;

/// The endlessly scrolling base.
#[derive(Clone, Copy, Debug)]
pub struct Base {
    /// Top y of the base.
    pub y: f32,
    /// x of the first tile.
    pub x1: f32,
    /// x of the second tile.
    pub x2: f32,
}

impl Base {
    /// Create a base whose top sits at `y`.
    pub fn new(y: f32) -> Self {
        Self {
            y,
            x1: 0.0,
            x2: WIDTH,
        }
    }

    /// Scroll both tiles one step, wrapping whichever has left the screen.
    pub fn update(&mut self) {
        self.x1 -= VELOCITY;
        self.x2 -= VELOCITY;

        if self.x1 + WIDTH < 0.0 {
            self.x1 = self.x2 + WIDTH;
        }
        if self.x2 + WIDTH < 0.0 {
            self.x2 = self.x1 + WIDTH;
        }
    }
}
