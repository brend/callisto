# Playdate Bindings (Callisto)

Shared extern module declarations for Playdate SDK access from Callisto projects.

## Layout

- `src/playdate.cal`: root `playdate.*` functions
- `src/playdate/graphics.cal`: graphics APIs
- `src/playdate/graphics/sprite.cal`: sprite APIs
- `src/playdate/timer.cal`: timer APIs

## Usage

Add this directory as a module root in your project `callisto.toml`:

```toml
module_roots = ["src", "../playdate_bindings/src"]
```

Then import the modules you need:

```callisto
import playdate
import playdate.graphics
import playdate.graphics.sprite
import playdate.timer
```
