//! All drawing lives here, keeping [`crate::game`] free of macroquad.
//!
//! Two render paths are supported, chosen at startup by [`Assets::load`]:
//!
//! * **Sprites** — the original `imgs/*.png` art, loaded with
//!   [`macroquad::texture::load_texture`] and scaled `2×` to mirror the
//!   original `pygame.transform.scale2x`. Pipes are flipped for the top half,
//!   the bird is animated and rotated by its tilt, and the base scrolls.
//! * **Primitives** — a dependency-free fallback (coloured shapes) used when
//!   any sprite is missing, so the game always runs.
//!
//! The diagnostic overlay (the orange "constrained region" lines), score, HUD
//! and game-over screen are drawn the same way in both paths.

use crate::game::Game;
use flappy_game::{pipe_in_front, Pipe, GROUND_Y};
use macroquad::prelude::*;
use macroquad::texture::load_texture;

// --- primitive palette (fallback path) --------------------------------------
const SKY: Color = Color::new(0.42, 0.74, 0.85, 1.0);
const PIPE_FILL: Color = Color::new(0.36, 0.71, 0.20, 1.0);
const PIPE_EDGE: Color = Color::new(0.20, 0.46, 0.11, 1.0);
const GROUND_FILL: Color = Color::new(0.87, 0.76, 0.42, 1.0);
const GROUND_EDGE: Color = Color::new(0.55, 0.43, 0.20, 1.0);
const BIRD_BODY: Color = Color::new(0.98, 0.82, 0.10, 1.0);
const BIRD_WING: Color = Color::new(0.90, 0.62, 0.07, 1.0);
const BEAK: Color = Color::new(0.93, 0.45, 0.13, 1.0);
const DIAGNOSTIC: Color = Color::new(1.0, 0.39, 0.0, 1.0);

const LIP_HEIGHT: f32 = 26.0;
const LIP_OVERHANG: f32 = 5.0;

/// Directory the sprite PNGs are loaded from, relative to the working
/// directory (the workspace root when launched via `cargo run`).
const ASSET_DIR: &str = "assets/imgs";
/// Factor matching the original `pygame.transform.scale2x`.
const SCALE2X: f32 = 2.0;
/// Number of simulation steps each animation frame is held (original `ANIMATION_TIME`).
const ANIMATION_TIME_STEPS: f64 = 5.0;
/// Tilt (deg) at or below which the bird is considered nose-diving.
const NOSE_DIVE_TILT: f32 = -80.0;

/// Rendering assets: either the loaded sprite set, or a request to draw
/// primitives.
pub enum Assets {
    Sprites(Sprites),
    Primitives,
}

/// The loaded sprite textures, mirroring the original `imgs/` set.
pub struct Sprites {
    bg: Texture2D,
    base: Texture2D,
    pipe: Texture2D,
    birds: [Texture2D; 3],
}

impl Assets {
    /// Attempt to load the sprite set; fall back to primitive rendering if any
    /// file is missing or fails to decode. Loading is async because
    /// [`load_texture`] is.
    pub async fn load() -> Self {
        match Sprites::try_load().await {
            Some(sprites) => {
                // Pixel-art: keep edges crisp when scaling instead of blurring.
                sprites.apply_filter(FilterMode::Nearest);
                Assets::Sprites(sprites)
            }
            None => Assets::Primitives,
        }
    }

    /// Whether the sprite path is active (used to tweak the HUD hint).
    pub fn uses_sprites(&self) -> bool {
        matches!(self, Assets::Sprites(_))
    }
}

impl Sprites {
    /// Load every sprite, returning `None` if any is unavailable.
    async fn try_load() -> Option<Self> {
        let bg = load_one("bg.png").await?;
        let base = load_one("base.png").await?;
        let pipe = load_one("pipe.png").await?;
        let bird1 = load_one("bird1.png").await?;
        let bird2 = load_one("bird2.png").await?;
        let bird3 = load_one("bird3.png").await?;
        Some(Self {
            bg,
            base,
            pipe,
            birds: [bird1, bird2, bird3],
        })
    }

    fn apply_filter(&self, mode: FilterMode) {
        self.bg.set_filter(mode);
        self.base.set_filter(mode);
        self.pipe.set_filter(mode);
        for bird in &self.birds {
            bird.set_filter(mode);
        }
    }
}

/// Load a single texture from [`ASSET_DIR`], swallowing any I/O/decode error.
async fn load_one(file: &str) -> Option<Texture2D> {
    load_texture(&format!("{ASSET_DIR}/{file}")).await.ok()
}

/// Draw the whole frame for the current game state.
pub fn draw(game: &Game, assets: &Assets, diagnostics: bool) {
    match assets {
        Assets::Sprites(sprites) => draw_world_sprites(game, sprites),
        Assets::Primitives => draw_world_primitives(game),
    }

    // Overlays shared by both render paths.
    if diagnostics {
        draw_diagnostics(game);
    }
    draw_score(game.score);
    draw_hud(diagnostics, assets.uses_sprites());
    if !game.alive {
        draw_game_over(game.score);
    }
}

// =============================================================================
// Sprite path
// =============================================================================

fn draw_world_sprites(game: &Game, s: &Sprites) {
    // Background, stretched to fill the window (the original blits it at 2×).
    draw_texture_ex(
        &s.bg,
        0.0,
        0.0,
        WHITE,
        DrawTextureParams {
            dest_size: Some(vec2(flappy_game::SCREEN_WIDTH, flappy_game::SCREEN_HEIGHT)),
            ..Default::default()
        },
    );

    for pipe in &game.pipes {
        draw_pipe_sprite(pipe, &s.pipe);
    }
    draw_base_sprite(game, &s.base);
    draw_bird_sprite(game, &s.birds);
}

fn draw_pipe_sprite(pipe: &Pipe, tex: &Texture2D) {
    let w = flappy_game::pipe::WIDTH;
    let native_h = tex.height() * SCALE2X;

    // Upper pipe: flipped vertically, its mouth resting on the gap's top edge,
    // its body running up and off the top of the screen.
    let top_h = native_h.max(pipe.gap_top);
    draw_texture_ex(
        tex,
        pipe.x,
        pipe.gap_top - top_h,
        WHITE,
        DrawTextureParams {
            dest_size: Some(vec2(w, top_h)),
            flip_y: true,
            ..Default::default()
        },
    );

    // Lower pipe: mouth on the gap's bottom edge, body running down to the base.
    let bottom_cover = GROUND_Y - pipe.gap_bottom;
    let bottom_h = native_h.max(bottom_cover);
    draw_texture_ex(
        tex,
        pipe.x,
        pipe.gap_bottom,
        WHITE,
        DrawTextureParams {
            dest_size: Some(vec2(w, bottom_h)),
            ..Default::default()
        },
    );
}

fn draw_base_sprite(game: &Game, tex: &Texture2D) {
    // Two tiles, drawn at the tile width the scroll logic assumes so they abut
    // seamlessly. Height is the native 2× height, anchored at the base top and
    // allowed to overflow the bottom of the window (as in the original).
    let h = tex.height() * SCALE2X;
    for x in [game.base.x1, game.base.x2] {
        draw_texture_ex(
            tex,
            x,
            game.base.y,
            WHITE,
            DrawTextureParams {
                dest_size: Some(vec2(flappy_game::base::WIDTH, h)),
                ..Default::default()
            },
        );
    }
}

fn draw_bird_sprite(game: &Game, birds: &[Texture2D; 3]) {
    let tex = &birds[current_bird_frame(game.bird.tilt)];
    let b = game.bird.bounds();
    draw_texture_ex(
        tex,
        b.x,
        b.y,
        WHITE,
        DrawTextureParams {
            // Scale the sprite to the bird's collision box so visuals and
            // hitbox stay consistent with the controller's model.
            dest_size: Some(vec2(b.w, b.h)),
            // pygame tilt is counter-clockwise-positive; macroquad rotation is
            // clockwise-positive, so negate. Pivot defaults to the dest centre.
            rotation: -game.bird.tilt.to_radians(),
            ..Default::default()
        },
    );
}

/// Pick the animation frame, cycling `0 → 1 → 2 → 1` on a fixed timer. While
/// nose-diving the wing is held level (frame 1), exactly like the original.
fn current_bird_frame(tilt: f32) -> usize {
    if tilt <= NOSE_DIVE_TILT {
        return 1;
    }
    const SEQUENCE: [usize; 4] = [0, 1, 2, 1];
    let frame_duration = ANIMATION_TIME_STEPS / 30.0; // seconds per frame
    let phase = (get_time() / frame_duration) as usize % SEQUENCE.len();
    SEQUENCE[phase]
}

// =============================================================================
// Primitive path (fallback)
// =============================================================================

fn draw_world_primitives(game: &Game) {
    clear_background(SKY);
    for pipe in &game.pipes {
        draw_pipe(pipe);
    }
    draw_base(game);
    draw_bird(game);
}

fn draw_pipe(pipe: &Pipe) {
    let w = flappy_game::pipe::WIDTH;

    // Upper pipe body + lip.
    draw_rectangle(pipe.x, 0.0, w, pipe.gap_top, PIPE_FILL);
    draw_rectangle_lines(pipe.x, 0.0, w, pipe.gap_top, 3.0, PIPE_EDGE);
    draw_rectangle(
        pipe.x - LIP_OVERHANG,
        pipe.gap_top - LIP_HEIGHT,
        w + 2.0 * LIP_OVERHANG,
        LIP_HEIGHT,
        PIPE_FILL,
    );
    draw_rectangle_lines(
        pipe.x - LIP_OVERHANG,
        pipe.gap_top - LIP_HEIGHT,
        w + 2.0 * LIP_OVERHANG,
        LIP_HEIGHT,
        3.0,
        PIPE_EDGE,
    );

    // Lower pipe body + lip.
    let lower_h = GROUND_Y - pipe.gap_bottom;
    draw_rectangle(pipe.x, pipe.gap_bottom, w, lower_h, PIPE_FILL);
    draw_rectangle_lines(pipe.x, pipe.gap_bottom, w, lower_h, 3.0, PIPE_EDGE);
    draw_rectangle(
        pipe.x - LIP_OVERHANG,
        pipe.gap_bottom,
        w + 2.0 * LIP_OVERHANG,
        LIP_HEIGHT,
        PIPE_FILL,
    );
    draw_rectangle_lines(
        pipe.x - LIP_OVERHANG,
        pipe.gap_bottom,
        w + 2.0 * LIP_OVERHANG,
        LIP_HEIGHT,
        3.0,
        PIPE_EDGE,
    );
}

fn draw_base(game: &Game) {
    let h = flappy_game::SCREEN_HEIGHT - GROUND_Y;
    draw_rectangle(0.0, game.base.y, flappy_game::SCREEN_WIDTH, h, GROUND_FILL);
    draw_line(
        0.0,
        game.base.y,
        flappy_game::SCREEN_WIDTH,
        game.base.y,
        3.0,
        GROUND_EDGE,
    );
    // Diagonal hatching that scrolls with the base tiles, so motion is visible.
    let spacing = 24.0;
    for tile_x in [game.base.x1, game.base.x2] {
        let mut sx = tile_x;
        while sx < tile_x + flappy_game::base::WIDTH {
            if sx + 14.0 >= 0.0 && sx <= flappy_game::SCREEN_WIDTH {
                draw_line(sx, game.base.y + 4.0, sx + 14.0, game.base.y + h, 2.0, GROUND_EDGE);
            }
            sx += spacing;
        }
    }
}

fn draw_bird(game: &Game) {
    let (cx, cy, _) = game.bird.physical_position();
    let r = flappy_game::bird::HEIGHT / 2.0;

    // Body.
    draw_circle(cx, cy, r, BIRD_BODY);
    draw_circle_lines(cx, cy, r, 2.0, BIRD_WING);
    // Wing.
    draw_circle(cx - r * 0.2, cy + r * 0.1, r * 0.55, BIRD_WING);
    // Eye.
    draw_circle(cx + r * 0.45, cy - r * 0.35, r * 0.28, WHITE);
    draw_circle(cx + r * 0.55, cy - r * 0.35, r * 0.12, BLACK);
    // Beak (a small triangle pointing right).
    draw_triangle(
        vec2(cx + r * 0.8, cy - r * 0.15),
        vec2(cx + r * 0.8, cy + r * 0.25),
        vec2(cx + r * 1.5, cy + r * 0.05),
        BEAK,
    );
}

// =============================================================================
// Shared overlays
// =============================================================================

/// Orange lines marking the gap the controller is steering through — a direct
/// port of the original `draw_diagnostics`.
fn draw_diagnostics(game: &Game) {
    let front = pipe_in_front(&game.bird, &game.pipes);
    let (cx, _, _) = game.bird.physical_position();
    let x_left = cx - 40.0;
    let x_right = front.right();

    draw_line(x_left, front.gap_top, x_right, front.gap_top, 4.0, DIAGNOSTIC);
    draw_line(
        x_left,
        front.gap_bottom,
        x_right,
        front.gap_bottom,
        4.0,
        DIAGNOSTIC,
    );
}

fn draw_score(score: u32) {
    let text = format!("Score: {score}");
    // Drop-shadow so the score stays readable over either background.
    draw_text(&text, 13.0, 41.0, 36.0, Color::new(0.0, 0.0, 0.0, 0.45));
    draw_text(&text, 12.0, 40.0, 36.0, WHITE);
}

fn draw_hud(diagnostics: bool, uses_sprites: bool) {
    let hint = if diagnostics {
        "D: hide guides   R: restart   Esc: quit"
    } else {
        "D: show guides   R: restart   Esc: quit"
    };
    draw_text(hint, 12.0, flappy_game::SCREEN_HEIGHT - 16.0, 20.0, WHITE);

    if !uses_sprites {
        draw_text(
            "primitives - drop sprites into assets/imgs/ for the original art",
            12.0,
            flappy_game::SCREEN_HEIGHT - 38.0,
            18.0,
            Color::new(1.0, 1.0, 1.0, 0.75),
        );
    }
}

fn draw_game_over(score: u32) {
    draw_rectangle(
        0.0,
        0.0,
        flappy_game::SCREEN_WIDTH,
        flappy_game::SCREEN_HEIGHT,
        Color::new(0.0, 0.0, 0.0, 0.45),
    );
    let title = "GAME OVER";
    let tw = measure_text(title, None, 56, 1.0).width;
    draw_text(
        title,
        (flappy_game::SCREEN_WIDTH - tw) / 2.0,
        340.0,
        56.0,
        WHITE,
    );

    let sub = format!("Score: {score}");
    let sw = measure_text(&sub, None, 36, 1.0).width;
    draw_text(&sub, (flappy_game::SCREEN_WIDTH - sw) / 2.0, 390.0, 36.0, WHITE);

    let prompt = "press R to play again";
    let pw = measure_text(prompt, None, 26, 1.0).width;
    draw_text(
        prompt,
        (flappy_game::SCREEN_WIDTH - pw) / 2.0,
        430.0,
        26.0,
        WHITE,
    );
}
