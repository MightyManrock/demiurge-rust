# System View & Planet Navigation — Design Spec
*2026-06-16*

## Overview

A two-mode Godot scene: **system view** shows the Oros star system with orbital motion; **planet view** shows the full planet with the existing shader-composited texture. A camera tween transitions between them. Both modes live in a single scene — no scene switching required.

Scope: Oros + its star only. Additional bodies (moons, other planets) are future work.

---

## Scene Structure

```
Node3D  [planet_view.gd]
├── SystemView  [Node3D — visible in system view, hidden in planet view]
│   ├── Star  [MeshInstance3D — SphereMesh, emissive core shader]
│   ├── StarHaloInner  [MeshInstance3D — larger sphere, transparent emissive, cull_front]
│   ├── StarHaloOuter  [MeshInstance3D — even larger sphere, faint emissive, cull_front]
│   ├── OrbitalRing  [MeshInstance3D — TorusMesh, faint emissive, static]
│   └── OrosProxy  [MeshInstance3D — small SphereMesh, annual composite texture]
├── PlanetSphere  [MeshInstance3D — existing full-res sphere, hidden in system view]
├── Camera3D
├── DirectionalLight3D  [fixed, color matched to star kind]
└── UI  [CanvasLayer]
    ├── ZoomInButton  [visible when Oros is selected in system view]
    └── BackButton  [visible in planet view]
```

`OrosProxy` uses the annual composite texture already produced by `renderer.annual_texture()` — no new Rust work. `PlanetSphere` is the existing full-res sphere; it holds the `ShaderMaterial` in planet view and the annual composite `StandardMaterial3D` in system view.

**Planet selection**: `OrosProxy` has an `Area3D` + `CollisionShape3D` child. A raycast from the camera on left-click tests against it. On hit, `OrosProxy` is marked selected (visual highlight TBD — simple emissive tint is sufficient) and `ZoomInButton` becomes visible. Clicking empty space deselects.

---

## System View Camera

- **Projection**: `PROJECTION_ORTHOGONAL` — true orthographic, no perspective distortion
- **Angle**: fixed rotation, looking down at ~60° from horizontal so the orbital plane reads clearly; rotation never changes
- **Pan**: right-click drag (or middle-mouse drag) translates camera position along the XZ plane
- **Zoom**: scroll wheel adjusts `orthographic_size`, clamped to a min/max range
- **Default**: centered on the star at startup
- **Implemented in**: `_process_system_camera()` in `planet_view.gd`, only runs when `_in_planet_view == false`

---

## Orbital Motion

Oros follows a circular orbit driven by the game tick counter.

- **Orbit radius**: a fixed visual constant (e.g. `8.0` units) — not physically scaled to AU
- **Angular position**: `angle = (tick / 536.0) * TAU` (536 = Oros orbital period in days)
- **OrosProxy position each frame**: `Vector3(cos(angle) * radius, 0.0, sin(angle) * radius)`
- **OrbitalRing**: a static `TorusMesh` at `y=0` sized to the orbit radius, set once at `_ready()`, never moves
- **Star**: centered at origin, static

The tick counter from the existing `_process()` loop drives the orbital angle directly — no separate timer.

---

## Star Appearance

Three concentric emissive spheres. All use `flags_unshaded = true`; halos use `cull_front` so only inner faces render.

| Node | Material | Notes |
|------|----------|-------|
| Star | Custom shader — radial gradient white → yellow → orange | Core body |
| StarHaloInner | `StandardMaterial3D`, semi-transparent emissive | ~1.4× star radius |
| StarHaloOuter | `StandardMaterial3D`, very faint emissive | ~2.2× star radius |

**Parameters from `Star` struct:**

| Field | Drives |
|-------|--------|
| `radius` | Core sphere scale |
| `luminosity` | Halo radius multiplier + emission energy |
| `kind` | Base color temperature (e.g. `YellowDwarf` → `#ffd040`) |

`DirectionalLight3D` color is set from the same kind → color mapping for consistent planet lighting.

---

## Camera Transition

Triggered by the **Zoom In** button (visible when Oros is selected). Reversed by the **Back** button.

**Zoom in sequence:**
1. Tick counter pauses
2. Camera switches to `PROJECTION_PERSPECTIVE`
3. `Tween` animates `camera.global_position` and `camera.rotation` from current system position to a fixed orbit point relative to Oros's *current* world position (~1.0–1.5 s, ease-in-out)
4. At ~50% tween progress: `SystemView.visible = false`, `PlanetSphere.visible = true`
5. On tween complete: `PlanetSphere` switches from annual `StandardMaterial3D` to `ShaderMaterial`; `_in_planet_view = true`; tick counter resumes

**Zoom out sequence** (Back button): reverse — swap material back, tween out, at completion switch to orthographic, show `SystemView`, resume ticks.

The tween target for zoom-in is computed from Oros's world position at the moment the button is clicked, so the camera flies toward wherever Oros is in its orbit.

---

## Planet View Navigation (Arcball)

Camera orbits a fixed point at Oros's world center. State: `yaw: float`, `pitch: float`, `orbit_radius: float`.

- **Mouse drag** (left button): delta X → yaw, delta Y → pitch
- **WASD**: W/S → pitch, A/D → yaw, same rate as drag
- **Scroll wheel**: adjusts `orbit_radius`, clamped between just-above-surface and planet-fills-~half-screen
- **Pitch clamped** to ±80° to prevent gimbal flip
- **Camera look target**: `camera.look_at(oros_world_center, Vector3.UP)` after every change
- **Active only when** `_in_planet_view == true` and the transition tween has fully completed
- **Implemented in**: `_process_planet_camera()` in `planet_view.gd`

---

## Out of Scope

- Realistic orbital scaling (AU distances)
- Elliptical orbits / orbital inclination
- Multiple planets or moons
- Planet selection sidebar (deferred — noted as future work)
- Flat map projection alternative for planet view
- Atmospheric / cloud layers
