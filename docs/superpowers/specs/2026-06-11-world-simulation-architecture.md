# World Simulation Architecture

**Date:** 2026-06-11
**Status:** Decided — not yet implemented

---

## Spatial Model

Planet surfaces are represented as **hex grids**. This gives real adjacency, real pathfinding, and real traversal costs without the overhead of continuous coordinate space. Entities have actual positions on the grid; terrain affects movement cost and danger; pathfinding is fallible and emergent (caravans get lost, turn back, learn the world over time).

Gas giants and similar bodies are implied by the *absence* of surface hex data combined with physical properties (high gravity, large size, no atmosphere tags).

## Level of Detail (LOD)

Only the world currently under the player's gaze runs at full fidelity. All other worlds run an abstract simulation and accumulate stale state. When the player shifts focus to a new world, the abstract model "snaps forward" to a plausible present state. Information about unobserved worlds is available but stale — dated to the last time that scope was actively observed.

## Scope Ladder

The player's attention operates at discrete scopes, each revealing a different simulation layer:

| Scope | What becomes visible |
|---|---|
| Universe | Galaxies, broad cosmic structure |
| Galaxy | Stellar events, hints of interstellar civilization |
| System | Planetary conditions, whispers of surface events |
| Planet | Full hex grid, moving entities, terrain, settlements |
| Direct contact | A single mortal's thoughts, intentions, current state (requires an active divine action — not passive observation) |

Everything below the current scope runs abstract and ages. The player may be watching a planet while having stale system-level data.

## Surface Structure

Hex cells replace the previous named-region / point-crawl model. Each cell has:
- Terrain kind(s) and traversal cost
- Atmosphere and physical properties (where they differ from the parent planet)
- Entity positions (mortals, pops, settlements)

## Orbital Layer

Orbital installations (stations, rings, etc.) exist in a separate layer above the hex surface. They connect down to surface cells via specific edges — spaceports, tethers, drop points. The `PopLocationKind` field encodes the "exit cost" of a location: leaving a surface settlement on foot is different from leaving an orbital station without a ship.

## Information Model

Entities carry **last-known state** rather than live state when outside the active scope. A mortal's position, a faction's strength, a planet's political situation — all are snapshots from the last time the player observed at the relevant scope. Acting on stale information is a meaningful risk and part of the cost of inattention.
