# Hex Grid Design

**Date:** 2026-06-11
**Status:** Decided — not yet implemented

---

## Hex Size

Planet surfaces use a hex grid with cells approximately **100km flat-to-flat**. At this scale an Earth-sized planet has ~59,000 hexes — granular enough for meaningful geography, manageable enough to simulate across many worlds.

Small features (islands, peaks, sacred sites) are not guaranteed their own hex. They exist as sub-hex landmarks or are reflected in the hex's terrain composition.

## Coordinate System

Hexes use standard **axial coordinates** `(q, r)`. This makes neighbor lookup trivial and is the industry-standard approach for hex grids.

## Terrain Composition

Each hex carries a `HashMap<TerrainKind, f64>` where values represent the approximate fraction of the hex covered by each terrain kind, summing to 1.0. A hex is rarely a single pure terrain — a mostly-ocean hex with a small island is something like `{OpenWater: 0.85, Coastal: 0.10, Rocky: 0.05}`.

```rust
pub enum TerrainKind {
    Forest,
    Grassland,
    Wetland,
    Desert,
    Tundra,
    Mountain,
    Coastal,
    OpenWater,
    IceSheet,
    AshField,
    CaveSystem,
    Ruins,
    ArtificialInterior,
    ArtificialExterior,
}
```

## Elevation

Elevation is a qualitative tier rather than a raw number, prioritizing gameplay legibility over precision:

```rust
pub enum ElevationKind {
    Depression,    // below surrounding terrain — valleys, basins, craters
    Flat,
    Rough,         // uneven but passable
    Hilly,
    Mountainous,
    Peak,          // extreme — near-impassable without specific capability
}
```

## Rivers

Rivers are an optional feature on a hex. They come in two forms: a river *crossing* the hex (entering one edge, exiting another) or the hex *being* the river — for very wide rivers like the Amazon that span multiple hexes.

```rust
pub enum HexRiver {
    Crossing {
        entry: HexEdge,
        exit: HexEdge,
        width: RiverWidth,
        roughness: RoughnessKind,
        liquid: Solvent,        // reused/extended from bio.rs
    },
    Body {
        roughness: RoughnessKind,
        liquid: Solvent,
    },
}

pub enum HexEdge { N, NE, SE, S, SW, NW }

pub enum RiverWidth { Narrow, Moderate, Wide, Vast }

pub enum RoughnessKind { Calm, Moderate, Rapids, Violent }
```

`Solvent` is drawn from (and may extend) the existing `Solvent` enum in `bio.rs`. What a river *means* for traversal depends on the species crossing it, not the river itself.

The `HexEdge` on a crossing is mechanically significant: entities traveling through the hex must cross the river at the edge where it intersects their path.

## Subsurface Layer

Any hex — surface or river body — may have a subsurface layer. This represents cave systems, underwater terrain, subterranean civilizations, aquifer settlements, etc. It carries its own terrain composition and can host its own settlements and inhabitants.

```rust
pub struct Subsurface {
    pub terrain: HashMap<TerrainKind, f64>,
    pub atmo: Option<Vec<AtmosphereTag>>,
    pub settlements: Vec<Settlement>,
    pub landmarks: Vec<Landmark>,
}
```

## Planetary Overrides

Most hexes inherit atmosphere and gravity from their parent planet. These fields are only present on a hex when they differ — a pressurized underground habitat, a anomalous magnetic zone, etc.

## Sub-Hex Positioning

Landmarks, settlements, and traveling entities all share a common `HexPos` struct representing a normalized position within the hex:

```rust
pub struct HexPos {
    pub x: f64,   // 0.0 = left edge, 1.0 = right edge
    pub y: f64,   // 0.0 = bottom edge, 1.0 = top edge
}
```

This is not a true coordinate system — cross-hex math is done at the hex level. `HexPos` exists to give the UI enough information to render entity and settlement indicators in roughly the right place on the globe.

When a traveling entity reaches a hex edge (`x` or `y` at 0.0 or 1.0), they transition to the neighboring hex with the relevant axis flipped.

## Landmarks and Settlements

Both are sub-hex features located via `HexPos`. They are distinct structs — a landmark is a notable geographic or cultural feature (a peak, a ruin, a sacred site), while a settlement is an inhabited place with population and faction data. Both `Landmark` and `Settlement` appear in `Vec`s on `HexCell` and also on `Subsurface`.

## HexCell Summary

```rust
pub struct HexCell {
    pub q: i32,
    pub r: i32,
    pub elevation: ElevationKind,
    pub terrain: HashMap<TerrainKind, f64>,
    pub river: Option<HexRiver>,
    pub subsurface: Option<Subsurface>,
    pub atmo: Option<Vec<AtmosphereTag>>,       // None = inherit from planet
    pub gravity: Option<f64>,                   // None = inherit from planet
    pub landmarks: Vec<Landmark>,
    pub settlements: Vec<Settlement>,
}
```

## Planet-Level Hex Map

A planet's surface is a `Vec<HexCell>`. The orbital layer (stations, rings, tethers) is separate and connects to surface hexes via specific edges — spaceports, drop points, tethers — represented as references into the surface hex vec.
