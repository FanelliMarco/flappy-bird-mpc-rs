# Flappy Bird MPC — Rust port

A Flappy Bird clone flown entirely by a **Model Predictive Control (MPC)**
policy. This is an idiomatic Rust port of the original Python project
(CVXPY + Gurobi + pygame), restructured as a **Cargo workspace** of small,
focused crates.

The headline change: the controller no longer needs Gurobi (or any external
solver). The original optimisation problem is solved **exactly** in
dependency-free Rust — see [How the MPC works](#how-the-mpc-works).

```
┌─────────────────────────────────────────────────────────────┐
│  flappy-sim (bin)        window, fixed 30 Hz loop, rendering  │
│      │            │                                           │
│      ▼            ▼                                           │
│  flappy-game   mpc-controller                                 │
│  (entities &   (exact mixed-integer                           │
│   physics)      quadratic program)                            │
└─────────────────────────────────────────────────────────────┘
```

## Workspace layout

| Crate            | Kind   | Responsibility                                                            | Deps          |
| ---------------- | ------ | ------------------------------------------------------------------------- | ------------- |
| `mpc-controller` | lib    | The MPC policy. Pure logic, no I/O, `#![forbid(unsafe_code)]`.            | none          |
| `flappy-game`    | lib    | Game entities (bird, pipes, base), physics, AABB collision. Render-free.  | `rand`        |
| `flappy-sim`     | bin    | Window, input, fixed-timestep loop, and macroquad rendering.              | `macroquad`   |

```
flappy-bird-mpc-rs/
├── Cargo.toml                  # workspace manifest (shared deps & profiles)
├── crates/
│   ├── mpc-controller/
│   │   └── src/lib.rs          # exact MIQP solver + tests
│   ├── flappy-game/
│   │   └── src/{lib,bird,pipe,base,geometry}.rs
│   └── flappy-sim/
│       └── src/{main,game,render}.rs
└── assets/imgs/                # optional; the renderer uses primitives
```

## Running it

Requires a recent stable Rust toolchain (1.80+ recommended).

```bash
cargo run -p flappy-sim --release
```

A 500×800 window opens and the MPC immediately starts playing.

| Key   | Action                              |
| ----- | ----------------------------------- |
| `D`   | toggle the orange diagnostic guides |
| `R`   | restart after a crash               |
| `Esc` | quit                                |

Run the test suite (covers the controller and the game logic):

```bash
cargo test
```

> **Linux build note:** macroquad needs system OpenGL/X11 (or Wayland) dev
> libraries to link the binary, e.g. on Debian/Ubuntu:
> `sudo apt install libx11-dev libxi-dev libgl1-mesa-dev libasound2-dev`.
> The `mpc-controller` and `flappy-game` crates have no such requirement.

## How the MPC works

Each step the controller is asked one question: *flap, or don't?* The original
framed this as a **mixed-integer quadratic program**:

```text
variables : x ∈ ℝ^{3×(N+1)}   state trajectory (x-pos, y-pos, vertical velocity)
            u ∈ {0,1}^N        flap decisions (boolean)
            ε ∈ ℝ^{2×N}        slacks on the gap constraints (≥ 0)

minimise  Σ (yᵢ − target)²  +  1e8 · Σ ε
where     target = (bottom-of-gap) − margin

subject to
  x-dynamics : xᵢ₊₁  = xᵢ + 5
  y-dynamics : yᵢ₊₁  = yᵢ + vyᵢ
  vy-dynamics: vyᵢ₊₁ = −20         if uᵢ = 1   (flap)
               vyᵢ₊₁ = vyᵢ + 2      if uᵢ = 0   (gravity)
  hard bounds: 0 ≤ yᵢ₊₁ ≤ 730
  soft gap   : yᵢ₊₁ ≤ bottom − margin + ε   and   yᵢ₊₁ ≥ top + margin − ε
```

### Why no solver is needed

For a **fixed** flap sequence `u`, the equality dynamics determine *every*
state uniquely — there is no remaining continuous freedom except the slacks,
which are minimised in closed form as `ε = max(0, violation)`. The problem
therefore reduces to choosing the best of the `2^N` flap sequences. With the
original horizon `N = 2`, that is just four candidates.

`mpc-controller` enumerates them, evaluates the exact objective for each, and
applies the first action of the cheapest plan (receding horizon). This returns
the **same global optimum** Gurobi would compute, with zero external
dependencies and fully portable code. Enumeration is exponential in the
horizon, so it suits the small horizons MPC-for-Flappy-Bird uses.

## What changed from the Python version, and why

These are deliberate adaptations; the physics, control logic, and game rules
are otherwise faithful to the original.

- **Solver → exact enumeration.** CVXPY/Gurobi replaced by the closed-form MIQP
  solution above. Identical decisions, no install friction.
- **Sprites loaded if present, primitives otherwise.** The renderer calls
  `macroquad::texture::load_texture` for the original `imgs/*.png` set
  (`bg`, `base`, `pipe`, `bird1..3`) from `assets/imgs/`, scaling them `2×`
  like the original `pygame.transform.scale2x`: pipes are flipped for the top
  half, the bird is animated (`0→1→2→1`) and rotated by its tilt, and the base
  scrolls. If any sprite is missing it falls back to coloured primitives so the
  game always runs. Either way, collision uses axis-aligned bounding boxes
  (the original PNG masks aren't required), matching the controller's model.
- **Separation of concerns.** Control, game logic, and rendering are three
  crates so the first two are pure and unit-testable with no graphics stack.
- **Fixed-timestep loop.** The simulation steps at a fixed 30 Hz (the original
  `clock.tick(30)`), decoupled from the display refresh rate.

## Credits

The original Flappy clone was based on
[Tech With Tim's NEAT-Flappy-Bird](https://github.com/techwithtim/NEAT-Flappy-Bird);
the upstream MPC project replaced the neural controller with the CVXPY/Gurobi
program ported here.

## License

MIT — see [`LICENSE`](LICENSE).
