# Sprite assets (optional)

Drop the original Flappy Bird PNGs here and the game uses them automatically —
no code changes needed:

```
assets/imgs/
├── bg.png
├── base.png
├── pipe.png
├── bird1.png
├── bird2.png
└── bird3.png
```

`flappy-sim` loads them at startup with `macroquad::texture::load_texture`,
scales them 2× (mirroring the original `pygame.transform.scale2x`), flips the
top pipe, animates the bird through the three frames, and rotates it by its
tilt. The base scrolls as two tiles.

If any file is missing, the renderer falls back to coloured primitives, so the
game still builds and runs with this directory empty. Collision is
bounding-box based regardless, so the sprites are purely cosmetic.
