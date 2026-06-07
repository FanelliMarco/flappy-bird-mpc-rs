//! Model Predictive Control (MPC) controller for Flappy Bird.
//!
//! # Background
//!
//! The original project modelled the bird's flight as a **mixed-integer
//! quadratic program** (MIQP) and handed it to CVXPY + Gurobi every frame:
//!
//! ```text
//! variables : x ∈ ℝ^{3×(N+1)}   state trajectory  (x-pos, y-pos, vy)
//!             u ∈ {0,1}^N        jump decisions    (boolean)
//!             ε ∈ ℝ^{2×N}        slacks on the gap constraints (≥ 0)
//!
//! minimise  Σ_i (y_i − lowerᵍᵃᵖ + margin)²  +  1e8 · Σ ε
//! s.t.      x-dynamics : x_{i+1} = x_i + 5
//!           y-dynamics : y_{i+1} = y_i + vy_i
//!           vy-dynamics: vy_{i+1} = −20            if u_i = 1   (jump)
//!                        vy_{i+1} = vy_i + 2       if u_i = 0   (gravity)
//!           bounds     : 0 ≤ y_{i+1} ≤ 730
//!           gap (soft) : y_{i+1} ≤ upperᵇᵒᵘⁿᵈ + ε   and   y_{i+1} ≥ lowerᵇᵒᵘⁿᵈ − ε
//! ```
//!
//! (The original code's big-M constraints are exactly the two dynamics
//! branches above; we keep the same constants for parity.)
//!
//! # Why we don't need Gurobi
//!
//! For a fixed jump sequence `u`, every continuous state is *uniquely
//! determined* by the equality dynamics — there is no remaining continuous
//! freedom except the slacks, which are minimised in closed form
//! (`ε = max(0, violation)`). The problem therefore collapses to picking the
//! best of the `2^N` jump sequences. With the original horizon `N = 2` that is
//! four candidates, so we enumerate them and evaluate the exact cost. This
//! yields the *same global optimum* Gurobi would return, but in dependency-free,
//! portable Rust.
//!
//! See [`Controller::solve`] for the entry point.

#![forbid(unsafe_code)]

/// Velocity imparted by a single flap (screen-y grows downward, so it's negative).
pub const JUMP_VELOCITY: f64 = -20.0;
/// Per-step gravitational acceleration applied when not flapping.
pub const GRAVITY: f64 = 2.0;
/// Safety buffer (px) the bird keeps from each pipe edge — the original `ROBUST_MARGIN`.
pub const ROBUST_MARGIN: f64 = 20.0;
/// Lowest admissible y (top of the screen).
pub const Y_MIN: f64 = 0.0;
/// Highest admissible y (top of the base).
pub const Y_MAX: f64 = 730.0;

/// Weight on the slack variables — matches the original `1e8` penalty.
const SLACK_PENALTY: f64 = 1e8;
/// Penalty for leaving the hard screen bounds. Strictly dominates any slack
/// term so feasible (in-bounds) trajectories are always preferred.
const BOUND_PENALTY: f64 = 1e13;

/// Physical state of the bird as seen by the controller.
///
/// Coordinates are in screen pixels with y growing downward, mirroring the
/// original `bird.physical_position()` (centre of the sprite plus velocity).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BirdState {
    /// Horizontal position. Decoupled from the decision, kept for fidelity.
    pub x: f64,
    /// Vertical position of the bird's centre.
    pub y: f64,
    /// Vertical velocity.
    pub vy: f64,
}

impl BirdState {
    /// Convenience constructor.
    pub fn new(x: f64, y: f64, vy: f64) -> Self {
        Self { x, y, vy }
    }
}

/// Vertical extent of the gap in the pipe directly ahead of the bird.
///
/// Because screen-y grows downward, `lower` (the *bottom* edge of the gap) has
/// the larger numeric value and `upper` (the *top* edge) the smaller one. This
/// matches the original `limits = [pipe_bottom_y, pipe_top_y]`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GapLimits {
    /// Bottom edge of the gap (larger y). The original `limits[0]`.
    pub lower: f64,
    /// Top edge of the gap (smaller y). The original `limits[1]`.
    pub upper: f64,
}

impl GapLimits {
    /// Convenience constructor.
    pub fn new(lower: f64, upper: f64) -> Self {
        Self { lower, upper }
    }
}

/// A receding-horizon controller that decides whether the bird should flap.
#[derive(Clone, Copy, Debug)]
pub struct Controller {
    horizon: usize,
}

impl Default for Controller {
    fn default() -> Self {
        // The original used `Controller(N=2, n_states=3)`. `n_states` is fixed
        // by the dynamics, so only the horizon is configurable here.
        Self::new(2)
    }
}

impl Controller {
    /// Create a controller with the given prediction `horizon` (`N`).
    ///
    /// # Panics
    /// Panics if `horizon == 0` or if `2^horizon` would overflow the
    /// enumeration counter (`horizon >= 32`). The intended range is small
    /// (the original used `2`); enumeration is exponential in the horizon.
    pub fn new(horizon: usize) -> Self {
        assert!(horizon > 0, "MPC horizon must be at least 1");
        assert!(horizon < 32, "horizon too large for exhaustive enumeration");
        Self { horizon }
    }

    /// The prediction horizon `N`.
    pub fn horizon(&self) -> usize {
        self.horizon
    }

    /// Decide whether to flap *this* step.
    ///
    /// Returns `true` if the optimal plan flaps on the first step. This is the
    /// receding-horizon move: we compute an optimal `N`-step plan and apply
    /// only its first action, exactly like `controller.solve(...)` returning
    /// `int(self.u.value[0])` in the original.
    pub fn solve(&self, state: BirdState, limits: GapLimits) -> bool {
        self.plan(state, limits).flap_now
    }

    /// Compute the full optimal plan (useful for diagnostics and tests).
    pub fn plan(&self, state: BirdState, limits: GapLimits) -> Plan {
        let n = self.horizon;
        let mut best = Plan {
            flap_now: false,
            cost: f64::INFINITY,
            sequence: Vec::new(),
            feasible: false,
        };

        // Enumerate every jump sequence u ∈ {0,1}^N. Bit `i` of `mask` is u_i.
        for mask in 0u32..(1u32 << n) {
            let sequence: Vec<bool> = (0..n).map(|i| (mask >> i) & 1 == 1).collect();
            let (cost, feasible) = self.evaluate(state, limits, &sequence);

            // Strict `<` keeps the first-found optimum; since mask 0 (never
            // flap) is evaluated first, ties resolve toward *not* flapping,
            // which yields smoother play.
            if cost < best.cost {
                best = Plan {
                    flap_now: sequence[0],
                    cost,
                    sequence,
                    feasible,
                };
            }
        }

        best
    }

    /// Evaluate one candidate jump sequence, returning `(cost, feasible)`.
    ///
    /// `feasible` is `true` when the predicted trajectory stays within the hard
    /// screen bounds `[Y_MIN, Y_MAX]`. Out-of-bounds trajectories are not
    /// discarded outright (that could leave us with no plan); instead they take
    /// a dominating [`BOUND_PENALTY`] so an in-bounds plan always wins when one
    /// exists, matching the spirit of the original hard constraints.
    fn evaluate(&self, state: BirdState, limits: GapLimits, sequence: &[bool]) -> (f64, bool) {
        let target = limits.lower - ROBUST_MARGIN; // objective drives y → bottom of the gap
        let gap_top_bound = limits.upper + ROBUST_MARGIN; // y must stay ≥ this
        let gap_bottom_bound = limits.lower - ROBUST_MARGIN; // y must stay ≤ this

        let mut y = state.y;
        let mut vy = state.vy;
        let mut feasible = true;

        // i = 0 term of the tracking objective (constant across sequences, but
        // included so the reported cost matches the original objective value).
        let mut tracking = (y - target).powi(2);
        let mut slack_sum = 0.0;
        let mut bound_violations = 0.0;

        for &flap in sequence {
            // y-dynamics use the *current* velocity, then vy is updated:
            //   y_{i+1}  = y_i + vy_i
            //   vy_{i+1} = JUMP_VELOCITY if flap else vy_i + GRAVITY
            y += vy;
            vy = if flap { JUMP_VELOCITY } else { vy + GRAVITY };

            // Hard screen bounds.
            if y < Y_MIN {
                feasible = false;
                bound_violations += Y_MIN - y;
            } else if y > Y_MAX {
                feasible = false;
                bound_violations += y - Y_MAX;
            }

            // Soft gap constraints: slacks take exactly the constraint violation.
            slack_sum += (y - gap_bottom_bound).max(0.0); // ε for the lower bound
            slack_sum += (gap_top_bound - y).max(0.0); // ε for the upper bound

            // Tracking term for this predicted position.
            tracking += (y - target).powi(2);
        }

        let cost = tracking + SLACK_PENALTY * slack_sum + BOUND_PENALTY * bound_violations;
        (cost, feasible)
    }
}

/// The outcome of an MPC optimisation over the full horizon.
#[derive(Clone, Debug, PartialEq)]
pub struct Plan {
    /// Whether to flap on the first (immediately applied) step.
    pub flap_now: bool,
    /// Optimal objective value of the chosen plan.
    pub cost: f64,
    /// The full optimal jump sequence over the horizon.
    pub sequence: Vec<bool>,
    /// Whether the chosen plan stays within the hard screen bounds.
    pub feasible: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    // A representative gap: top edge at y=250, bottom edge at y=450 (gap = 200).
    fn gap() -> GapLimits {
        GapLimits::new(450.0, 250.0)
    }

    #[test]
    fn default_horizon_matches_original() {
        assert_eq!(Controller::default().horizon(), 2);
    }

    #[test]
    fn flaps_when_falling_below_the_gap() {
        // Bird well below the gap and still falling: it must climb, so flap.
        let c = Controller::default();
        let state = BirdState::new(230.0, 600.0, 10.0);
        assert!(c.solve(state, gap()), "should flap to recover from below");
    }

    #[test]
    fn does_not_flap_when_rising_above_the_gap() {
        // Bird above the gap and already rising: flapping would overshoot.
        let c = Controller::default();
        let state = BirdState::new(230.0, 200.0, -5.0);
        assert!(!c.solve(state, gap()), "should glide down into the gap");
    }

    #[test]
    fn plan_reports_full_sequence_of_horizon_length() {
        let c = Controller::new(2);
        let plan = c.plan(BirdState::new(230.0, 350.0, 0.0), gap());
        assert_eq!(plan.sequence.len(), 2);
        assert!(plan.cost.is_finite());
    }

    #[test]
    fn comfortable_in_gap_centre_prefers_to_glide() {
        // Sitting in the middle of the gap with no velocity: there is room to
        // fall toward the (lower) target, so the cheapest action is to glide
        // rather than flap upward. Note that sitting *exactly* on the lower
        // robust-margin line would instead trigger a flap, because drifting any
        // lower violates the soft gap constraint — that is correct MPC behaviour.
        let c = Controller::default();
        let centre = (250.0 + 450.0) / 2.0;
        let state = BirdState::new(230.0, centre, 0.0);
        assert!(!c.solve(state, gap()));
    }

    #[test]
    #[should_panic]
    fn rejects_zero_horizon() {
        let _ = Controller::new(0);
    }
}
