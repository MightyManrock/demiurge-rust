# World Simulation Architecture

**Date:** 2026-06-11
**Status:** Decided — not yet implemented

---

## Spatial Model

Planet surfaces are represented as a **stack of heat maps** over a continuous coordinate space. Entities have real floating-point positions; terrain is sampled from the appropriate heat maps at any point. There are no discrete cells — the world is a smooth field. See `2026-06-11-world-map-design.md` for the full heat map specification.

This replaces the earlier hex grid model. The previous hex grid design is archived at `2026-06-11-hex-grid-design.md` (superseded).

Gas giants and similar bodies are implied by the *absence* of surface map data combined with physical properties (high gravity, large size, no atmosphere tags).

## Spatial Indexing

Entity-to-entity queries ("what Pops are near this settlement", "who can interact this tick") use an **R-tree** (via the `rstar` crate). The tree indexes current tick positions of all entities on a surface. Fuzzy territory and influence fields are heat maps — sampled in O(1), no spatial index needed. Terrain queries are similarly direct heat map lookups.

## Time Model

The simulation is strictly **tick-based** with a default tick length of one day. Logic resolves at tick boundaries only.

**Time cannot be paused.** The tick timer always runs. The player may slow it to an arbitrarily low rate, but ticks will always eventually fire — the minimum rate is a design parameter, not zero. This is intentional: the advancement of time is a core constraint of the Demiurge's existence, not a UI convenience to be bypassed.

Visual rendering uses **fixed-timestep interpolation**. Each entity stores both `prev_position` and `current_position`. The renderer receives an `alpha` value (0.0→1.0) indicating progress through the current tick and displays `lerp(prev, current, alpha)`. This gives smooth visual motion at any tick rate without affecting simulation logic.

## Level of Detail (LOD)

Only the world currently under the player's gaze runs at full fidelity. All other worlds run an abstract simulation and accumulate stale state. When the player shifts focus to a new world, the abstract model "snaps forward" to a plausible present state. Information about unobserved worlds is available but stale — dated to the last time that scope was actively observed.

## Scope Ladder

The player's attention operates at discrete scopes, each revealing a different simulation layer:

| Scope | What becomes visible |
|---|---|
| Universe | Galaxies, broad cosmic structure |
| Galaxy | Stellar events, hints of interstellar civilization |
| System | Planetary conditions, whispers of surface events |
| Planet | Full surface map, moving entities, terrain, settlements |
| Direct contact | A single mortal's thoughts, intentions, current state (requires an active divine action — not passive observation) |

Everything below the current scope runs abstract and ages. The player may be watching a planet while having stale system-level data.

## Surface Structure

The planet surface is a coordinate plane with wraparound on the east-west axis. Features on the surface include:

- **Heat maps** — navigability, climate, biome, liquid, and others; sampled at any coordinate
- **POIs** — landmarks and settlements as point features with continuous coordinates, stored in a static R-tree
- **Entities** — Mortals and Pops with continuous positions, stored in a dynamic R-tree rebuilt each tick

Subsurface layers (cave systems, aquifers, underground civilizations) use the same model and are addressed separately from the surface.

## Orbital Layer

Orbital installations (stations, rings, etc.) exist in a separate layer above the surface. They connect down to surface coordinates via specific interfaces — spaceports, tethers, drop points. The `PopLocationKind` field encodes the "exit cost" of a location: leaving a surface settlement on foot is different from leaving an orbital station without a ship.

## Information Model

Entities carry **last-known state** rather than live state when outside the active scope. A mortal's position, a faction's strength, a planet's political situation — all are snapshots from the last time the player observed at the relevant scope. Acting on stale information is a meaningful risk and part of the cost of inattention.
