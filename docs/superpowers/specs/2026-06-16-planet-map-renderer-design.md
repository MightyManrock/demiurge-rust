# Planet Map Renderer — Design Spec
*2026-06-16*

## Overview

A `PlanetMapRenderer` GodotClass bridges the Rust planet generation pipeline and the Godot scene graph. It pre-computes stable planet data once, then evaluates seasonal appearance on demand at a configurable tick cadence. A LOD model keeps the rendering cost proportional to how closely the player is viewing the planet.

---

## LOD Model

Two texture tiers:

| LOD | Context | Texture | Update trigger |
|-----|---------|---------|----------------|
| System view | Planet seen from a distance | Annual composite | Major planet event only |
| Planet view | Player focused on the planet | Seasonal texture | Every `update_interval_ticks` |

The annual composite is a full-year average — it does not change unless something planet-scale happens (e.g., orbital shift, catastrophic geological event). What counts as a "major event" is left TBD pending the event system design; for now a dirty flag on `PlanetMapRenderer` gates a full reinitialize.

---

## Rust: `PlanetMapRenderer`

A `GodotClass` that owns the planet state and exposes texture generation to Godot.

**Stable data (computed once on `initialize()`):**
- Elevation field
- Ocean mask
- Region detection output

These do not change between seasonal updates. Recomputed only when the dirty flag is set.

**Methods exposed to Godot:**
- `initialize()` — runs the stable generation pipeline; must be called before any texture method
- `annual_texture() -> PackedByteArray` — returns pixel data for the full-year composite; safe to call once and cache
- `seasonal_texture(phase: f64) -> PackedByteArray` — evaluates the phasor system at `phase` ∈ [0, 1) and returns pixel data for that point in the orbital year

**Property exposed to Godot:**
- `update_interval_ticks: i64` — `orbital_period_ticks / N`, where N starts at 12; Godot uses this to schedule seasonal texture refreshes. N may be bumped to 24 after visual testing.

Pixel data is returned as a flat RGB `PackedByteArray` (width × height × 3 bytes). Godot assembles this into an `Image` and wraps it in an `ImageTexture`.

---

## Godot Side

- Maintains a tick counter
- Computes `season_phase = (tick % orbital_period_ticks) / orbital_period_ticks as f64`
- Every `update_interval_ticks`, calls `seasonal_texture(season_phase)`, constructs an `Image`, and swaps the sphere `MeshInstance3D`'s `StandardMaterial3D` albedo texture
- On LOD switch to system view, calls `annual_texture()` and pins that texture until the next major event

---

## Data Flow

```
tick counter (Godot)
    │
    ▼
season_phase: f64
    │
    ▼
PlanetMapRenderer::seasonal_texture(phase)   [Rust]
    │  evaluates phasor fields (temp, precip, hydrology)
    │  composites pixel data
    ▼
PackedByteArray  →  Image::create_from_data()  →  ImageTexture  →  sphere albedo
```

---

## Out of Scope

- Multi-planet support (Oros only for now)
- Interpolation between snapshots on the Godot side (may revisit if updates feel choppy)
- Night-side rendering, cloud layers, atmospheric scattering
- The event system that triggers major-event invalidation
