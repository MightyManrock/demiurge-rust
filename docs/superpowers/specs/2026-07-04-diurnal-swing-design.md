# Diurnal Temperature Swing — Design Spec

**Date:** 2026-07-04
**Status:** Approved, pending implementation
**Source doc:** `docs/diurnal-temperature.md`

## Goal

Model the day/night temperature gap on a planet and make species habitability
scoring aware of it. Currently `generate_temperature` produces what is
effectively a diurnal *mean*, and habitability scores every species against that
mean. This conceals the ecologically significant swing between the daytime peak
and the predawn trough — the effect that makes deserts cold at night. On Oros
(low liquid coverage, long rotation) the swing is large, and it is the physical
grounding for the Keth being nocturnal: the night is their only thermally
tolerable window.

This change makes the Keth's nocturnality a real mechanic. A nocturnal species
should score its temperature comfort against the *nighttime trough*, not the
mean. This is expected to redraw which regions of Oros look habitable.

## Scope

Full implementation:
1. Simulation: a derived diurnal-swing field and day/night temperature.
2. Scoring: activity-pattern-aware habitability (region and cell level).
3. Render: Day-Temp and Night-Temp map layers in the Godot planet view.

Out of scope: retuning the Keth `temp_range`. Per decision, only the *field*
they score against changes (mean → nighttime trough); their existing 22–30°C
range is kept so we can see how the map redraws before deciding whether to
retune.

## Components

### 1. Activity pattern on `Species` (`src/bio.rs`)

New enum and field:

```rust
#[derive(Debug, Default, Serialize, Deserialize)]
pub enum ActivityPattern {
    #[default]
    Diurnal,
    Nocturnal,
    Crepuscular,
}

// on struct Species:
#[serde(default)]
pub activity: ActivityPattern,
```

`Default = Diurnal` covers most species and, with `#[serde(default)]`, keeps any
existing serialized `Species` deserializable. The Keth definition in
`examples/heatmap_export.rs` is set to `ActivityPattern::Nocturnal`. The Keth
`temp_range` stays at `22.0..30.0`.

### 2. Diurnal swing field (`src/planet_gen.rs`)

A free function, computed per temperature snapshot so it is inherently seasonal
(mirrors how temperature / precipitation / hydrology already produce four
seasonal snapshots):

```rust
pub fn generate_diurnal_swing(
    temperature: &HeatMap,
    aridity: &HeatMap,
    params: &PlanetParams,
) -> HeatMap
```

Formula, in normalized-temperature units (`[0,1]`, same space as the
`temperature` field), so it composes with the existing Celsius mapping
`normalized_temp_to_celsius(t) = t * 70.0 - 15.0`:

```
swing[cell] = MAX_SWING * temp * dryness * rotation_factor

  temp             = temperature[cell]        // hotter cells swing more in absolute terms
  dryness          = aridity[cell]            // local; high aridity = high swing
  rotation_factor  = params.rotation_period.sqrt()   // Earth = 1.0, Oros ≈ 1.118; diminishing returns
  MAX_SWING        = 0.55 (const, tunable)    // ~0.5 norm ≈ 35°C ≈ Sahara at full dryness + heat
```

**Humidity proxy decision:** the *local* `aridity` field is used rather than the
global `precip_moisture` scalar. The scalar would give every cell on a planet the
same swing; local aridity makes deserts swing hard while wet regions stay mild,
which is the ecological point. `precip_moisture` still influences swing
indirectly because it feeds aridity generation upstream.

Derived day/night temperature (computed where needed, not stored as separate
fields):

```
temp_day[cell]   = (mean + swing/2).clamp(0.0, 1.0)
temp_night[cell] = (mean - swing/2).clamp(0.0, 1.0)
```

`MAX_SWING` and `rotation_factor` are calibration knobs; final values are tuned
by eye against the rendered Night-Temp layer.

### 3. Region aggregation (`Region` + `detect_regions`)

Add one field to `Region`:

```rust
pub mean_swing: f64,
```

Populated by passing the swing `HeatMap` as a new parameter to `detect_regions`
and averaging it over the region's cells exactly like the other `mean_*` fields.
Two helper methods on `Region`:

```rust
pub fn mean_temp_day(&self)   -> f64 { (self.mean_temp + self.mean_swing / 2.0).clamp(0.0, 1.0) }
pub fn mean_temp_night(&self) -> f64 { (self.mean_temp - self.mean_swing / 2.0).clamp(0.0, 1.0) }
```

### 4. Activity-aware scoring (`src/planet_gen.rs`)

Both `score_region_for_species` and `score_cell_for_species` select the
temperature they score by the species' activity pattern instead of always using
the mean:

```rust
let effective_temp_norm = match species.activity {
    ActivityPattern::Diurnal     => temp_day,     // mean + swing/2
    ActivityPattern::Nocturnal   => temp_night,   // mean - swing/2
    ActivityPattern::Crepuscular => mean,
};
let temp_c = normalized_temp_to_celsius(effective_temp_norm);
```

- `score_region_for_species` reads `region.mean_swing` and uses
  `mean_temp_day()` / `mean_temp_night()`.
- `score_cell_for_species` gains a `swing: f64` parameter; callers pass the swing
  value at that cell.

Callers to update: `examples/heatmap_export.rs` (cell-level habitability
renders), which must supply the swing value per cell.

### 5. Render layers (Godot)

- `PlanetMapRenderer` (`src/planet_renderer.rs`) stores
  `seasonal_swing: Vec<HeatMap>`, one per snapshot, each built as
  `generate_diurnal_swing(&seasonal_temps[i], &aridity, &params)` in
  `initialize()`.
- Two new `#[func]`s, following the `ice_texture(snapshot)` pattern:
  `temp_day_texture(snapshot: i32)` and `temp_night_texture(snapshot: i32)`. Each
  builds the derived day/night field for the snapshot and colors it through the
  existing `temperature_color`.
- A small render helper (e.g. `render_temp_field_layer`) colors a temperature
  field: land/ocean cells through `temperature_color`, matching the visual
  conventions of the existing temperature rendering.
- `godot/scripts/planet_view.gd` adds Day-Temp and Night-Temp as two selectable
  layers, reusing the ice layer's four-snapshot blend logic.

## Testing

Unit tests in `src/planet_gen.rs`:

- Swing is zero when inputs bottom out (zero aridity or zero temperature).
- `temp_night ≤ mean ≤ temp_day` for all cells.
- Clamping holds at extremes (mean near 0 or 1 does not push day/night out of
  `[0,1]`).
- A hot, dry, long-rotation cell (Oros desert profile) produces a swing in the
  expected ~30–35°C band after Celsius conversion.

Scoring test:

- A Nocturnal and a Diurnal species with identical ranges score *differently* on
  a high-swing cell and *identically* on a zero-swing cell.

## Expected outcome

Selecting the Night-Temp layer in the planet view, and re-running the
habitability export for the (now nocturnal) Keth, should light up regions that
looked marginal on the mean-temperature map — confirming the swing is
mechanically load-bearing rather than cosmetic.
