//! Flappy Bird, flown by a Model Predictive Controller.
//!
//! The MPC ([`mpc_controller`]) decides whether to flap each step; the game
//! logic ([`flappy_game`]) integrates the world; this binary renders it with
//! macroquad and drives a fixed 30 Hz simulation step, matching the original
//! `clock.tick(30)`.

#![forbid(unsafe_code)]

mod game;
mod render;

use game::Game;
use macroquad::prelude::*;

/// Logical simulation rate (Hz) — the original ran physics at 30 fps.
const SIM_HZ: f32 = 30.0;
const SIM_DT: f32 = 1.0 / SIM_HZ;

fn window_conf() -> Conf {
    Conf {
        window_title: "Flappy Bird MPC".to_owned(),
        window_width: flappy_game::SCREEN_WIDTH as i32,
        window_height: flappy_game::SCREEN_HEIGHT as i32,
        window_resizable: false,
        ..Default::default()
    }
}

#[macroquad::main(window_conf)]
async fn main() {
    // Try the original sprites; transparently falls back to primitive drawing
    // if `assets/imgs/*.png` are absent.
    let assets = render::Assets::load().await;
    let mut game = Game::new();
    let mut diagnostics = true;
    let mut accumulator = 0.0_f32;

    loop {
        // --- input -----------------------------------------------------------
        if is_key_pressed(KeyCode::Escape) {
            break;
        }
        if is_key_pressed(KeyCode::R) {
            game.reset();
            accumulator = 0.0;
        }
        if is_key_pressed(KeyCode::D) {
            diagnostics = !diagnostics;
        }

        // --- fixed-timestep simulation --------------------------------------
        // Decouple physics from the display refresh rate: accumulate real time
        // and step the world in fixed 1/30 s increments.
        accumulator += get_frame_time().min(0.25); // clamp to avoid spiral-of-death
        while accumulator >= SIM_DT {
            game.step();
            accumulator -= SIM_DT;
            if !game.alive {
                accumulator = 0.0;
                break;
            }
        }

        // --- render ----------------------------------------------------------
        render::draw(&game, &assets, diagnostics);
        next_frame().await;
    }
}
