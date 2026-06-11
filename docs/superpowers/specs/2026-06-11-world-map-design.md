# World Map Design

**Date:** 2026-06-11
**Status:** Decided — not yet implemented

Supersedes: `2026-06-11-hex-grid-design.md`

---

## Model Overview

A planet surface is a **stack of heat maps** over a **continuous coordinate plane**. There are no discrete cells. Terrain is sampled at any point; entities occupy floating-point positions; features of interest are placed as POIs at specific coordinates.

This keeps resolution as an internal implementation detail (the backing array dimensions) rather than a load-bearing design concept. Game logic never needs to know the resolution.

## Coordinate System

The surface is a normalized rectangle: `x ∈ [0.0, 1.0)`, `y ∈ [0.0, 1.0)`. The x-axis wraps (east-west); the y-axis does not (poles). Subsurface layers share this coordinate system but are addressed as a separate layer stack.

## Heat Map Stack

Each map is a 2D array of floats, sampled by bilinear interpolation at any coordinate. Maps are generated during world creation and may update over simulation time (climate drift, terrain changes from events, etc.).

### Navigability Map

The "default" traversal difficulty, used when an entity has no familiarity with an area. Values encode the nature of difficulty, not just a scalar:

- **Open** — plains, roads, calm water; low noise, predictable pathing
- **Rough** — uneven terrain; introduces noise to progress estimates
- **Disorienting** — dense forest, featureless tundra, fog-prone valleys; chance of angular divergence from intended heading or looping
- **Impassable** — sheer cliffs, violent surf, etc.; requires specific capability to cross

An entity's KB may carry a **familiarity map** for areas it has traversed. Familiarity attenuates noise, reduces divergence probability in disorienting terrain, and can reveal routes through otherwise impassable features (a known mountain pass, a tunnel entrance). Familiarity is earned through experience, not granted.

### Climate Map

Encodes yearly weather and temperature cycles. Informs:
- Seasonal temperature ranges
- Precipitation patterns
- Extreme weather probability
- Secondary effects on navigability (frozen passes, flooded plains, sandstorm corridors)
- Solvent availability and seasonal crop yield potential

### Biome Map

Encodes ecosystem character — not just vegetation but the full ecological profile. Informs:
- Foraging and hunting resource availability
- Secondary navigability effects (dense undergrowth, open savanna)
- Natural hazard distribution (predator territories, toxic flora zones)

### Liquid Map

Encodes the distribution and character of surface liquids:
- River presence, width, and roughness
- Ocean and lake coverage
- Current direction and intensity (relevant for water travel)
- Liquid type (draws from `Solvent` enum — not all rivers are water)

Rivers wide enough to span a meaningful area are simply encoded as high-value liquid map regions rather than special-cased features.

### Additional Maps (anticipated)

- **Geological activity** — volcanic zones, fault lines, erosion patterns
- **Magical/exotic flux** — for worlds where ambient energy fields affect actions or entity behavior

## Points of Interest

Landmarks and settlements are **point features** with continuous coordinates. They are not tied to any cell. A settlement exists at `(0.42, 0.71)` and is what it is regardless of what the underlying terrain samples happen to be at that point.

POIs are stored in a static R-tree for fast spatial queries ("what settlements are within travel distance of this point"). The R-tree is rebuilt only when POIs are added or removed, which is infrequent.

## Entity Positions

Mortals and Pops carry continuous `(x, y)` coordinates. Each entity also stores its `prev_position` from the prior tick for visual interpolation (see architecture doc).

Entity positions are indexed in a **dynamic R-tree** (via `rstar`). This tree is rebuilt each tick after positions are resolved. Queries against it ("who can interact with whom this tick") always operate on fully resolved current-tick positions.

## Faction and Civilization Territory

Faction and civilization influence is **fuzzy** — represented as influence heat maps (one per active polity) rather than hard borders. Nominal control at a point is the argmax of all influence values there. Disputed zones, gradients, and contested frontiers emerge naturally from overlapping influence fields.

Influence maps update as factions grow, contract, or collapse. Querying "which entities fall under faction X's influence" means checking whether X's influence map exceeds a threshold at each entity's position — more expensive than a hard-border lookup, but a more honest model of how premodern (and many postmodern) political control actually works.

## Subsurface Layers

Any region of the surface may have a corresponding subsurface layer — cave systems, aquifers, underground civilizations, lava tubes. The subsurface uses the same heat map + continuous coordinate model, addressed as a separate layer. Transitions between surface and subsurface are POIs (cave entrances, sinkholes, constructed tunnels).

A subsurface layer may have its own atmosphere tags and gravity modifier where they differ from the surface.

## Planetary Overrides

Most of the surface inherits atmosphere and gravity from the parent planet. Local overrides (pressurized dome, anomalous gravity well) are encoded as POI-scoped properties or small regional heat maps, not as per-point fields on the main maps.
