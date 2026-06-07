//! Pure, render-free game logic for Flappy Bird.
//!
//! This crate owns the simulation entities — [`bird::Bird`], [`pipe::Pipe`],
//! [`base::Base`] — their physics, and collision via [`geometry::Aabb`]. It has
//! no dependency on any graphics library or on the controller, so it can be
//! unit-tested and reused headlessly. The [`flappy-sim`] binary wires it
//! together with the [`mpc_controller`] and a renderer.
//!
//! [`flappy-sim`]: ../flappy_sim/index.html
//! [`mpc_controller`]: ../mpc_controller/index.html

#![forbid(unsafe_code)]

pub mod base;
pub mod bird;
pub mod geometry;
pub mod pipe;

pub use base::Base;
pub use bird::Bird;
pub use geometry::Aabb;
pub use pipe::Pipe;

/// Logical playfield width in pixels.
pub const SCREEN_WIDTH: f32 = 500.0;
/// Logical playfield height in pixels.
pub const SCREEN_HEIGHT: f32 = 800.0;
/// Y of the top of the base / ground.
pub const GROUND_Y: f32 = 730.0;
/// Fixed x at which the bird is held while the world scrolls past it.
pub const BIRD_X: f32 = 230.0;
/// Initial y of the bird.
pub const BIRD_START_Y: f32 = 350.0;
/// x at which new pipes spawn.
pub const PIPE_SPAWN_X: f32 = 600.0;

/// Pick the pipe the bird must clear next.
///
/// Faithful to the original `pipe_in_front`: while the bird is still to the
/// left of the first pipe's right edge, that pipe is the target; otherwise the
/// most recently spawned pipe is.
pub fn pipe_in_front<'a>(bird: &Bird, pipes: &'a [Pipe]) -> &'a Pipe {
    debug_assert!(!pipes.is_empty(), "there must always be at least one pipe");
    if bird.x < pipes[0].right() {
        &pipes[0]
    } else {
        pipes.last().unwrap()
    }
}
