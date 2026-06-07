//! Game state and the fixed-step update that ties the entities to the MPC.
//!
//! This module is renderer-agnostic: it advances the simulation but draws
//! nothing. See [`crate::render`] for drawing.

use flappy_game::{
    pipe_in_front, Base, Bird, Pipe, BIRD_START_Y, BIRD_X, GROUND_Y, PIPE_SPAWN_X,
};
use mpc_controller::{BirdState, Controller, GapLimits};

/// The complete state of one play-through.
pub struct Game {
    pub bird: Bird,
    pub base: Base,
    pub pipes: Vec<Pipe>,
    pub controller: Controller,
    pub score: u32,
    pub alive: bool,
    /// Whether the controller flapped on the most recent step (for visuals).
    pub last_flap: bool,
    /// The gap limits the controller saw last step (for diagnostics overlay).
    pub last_limits: GapLimits,
}

impl Game {
    /// Start a fresh game, matching the original `main()` setup.
    pub fn new() -> Self {
        Self {
            bird: Bird::new(BIRD_X, BIRD_START_Y),
            base: Base::new(GROUND_Y),
            pipes: vec![Pipe::new(PIPE_SPAWN_X)],
            controller: Controller::default(),
            score: 0,
            alive: true,
            last_flap: false,
            last_limits: GapLimits::new(GROUND_Y as f64, 0.0),
        }
    }

    /// Reset to a brand-new game in place.
    pub fn reset(&mut self) {
        *self = Game::new();
    }

    /// Advance one fixed timestep (the original ran at 30 Hz).
    ///
    /// Order of operations mirrors the original loop: query the controller,
    /// apply the flap, integrate the bird, then resolve pipes and scoring.
    pub fn step(&mut self) {
        if !self.alive {
            return;
        }

        // 1. Controller decides whether to flap, given the bird's state and the
        //    near edge of the pipe ahead.
        let (cx, cy, vy) = self.bird.physical_position();
        let front = pipe_in_front(&self.bird, &self.pipes);
        let (lower, upper) = front.controller_limits();
        let limits = GapLimits::new(lower as f64, upper as f64);
        self.last_limits = limits;

        let state = BirdState::new(cx as f64, cy as f64, vy as f64);
        self.last_flap = self.controller.solve(state, limits);

        // 2. Apply the action and integrate the bird.
        if self.last_flap {
            self.bird.jump();
        }
        self.bird.update();

        // 3. Resolve pipes: collisions, scoring, spawning, despawning.
        let mut add_pipe = false;
        let mut remove = Vec::new();
        for (i, pipe) in self.pipes.iter_mut().enumerate() {
            if pipe.collides(&self.bird) {
                self.alive = false;
            }
            // Count the pipe as cleared once its left edge passes the bird.
            if !pipe.passed && pipe.x < self.bird.x {
                pipe.passed = true;
                add_pipe = true;
            }
            if pipe.right() < 0.0 {
                remove.push(i);
            }
            pipe.update();
        }

        if add_pipe {
            self.score += 1;
            self.pipes.push(Pipe::new(PIPE_SPAWN_X));
        }
        // Remove despawned pipes back-to-front so indices stay valid.
        for &i in remove.iter().rev() {
            self.pipes.remove(i);
        }

        // 4. Death by hitting the ground or the ceiling.
        if self.bird.bounds().bottom() > GROUND_Y || self.bird.y < 0.0 {
            self.alive = false;
        }

        // 5. Scroll the ground.
        self.base.update();
    }
}

impl Default for Game {
    fn default() -> Self {
        Self::new()
    }
}
