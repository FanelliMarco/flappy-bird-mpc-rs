//! A minimal axis-aligned bounding box used for collision tests.
//!
//! The original game used pixel-perfect masks (`pygame.mask`). We render with
//! primitives instead of the original sprites, so an AABB is both the natural
//! and the faithful choice for collision here.

/// An axis-aligned rectangle in screen space (y grows downward).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Aabb {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl Aabb {
    /// Construct a box from its top-left corner and size.
    pub fn new(x: f32, y: f32, w: f32, h: f32) -> Self {
        Self { x, y, w, h }
    }

    /// Right edge (`x + w`).
    pub fn right(&self) -> f32 {
        self.x + self.w
    }

    /// Bottom edge (`y + h`).
    pub fn bottom(&self) -> f32 {
        self.y + self.h
    }

    /// Whether this box overlaps `other` (touching edges do not count).
    pub fn intersects(&self, other: &Aabb) -> bool {
        self.x < other.right()
            && other.x < self.right()
            && self.y < other.bottom()
            && other.y < self.bottom()
    }
}
