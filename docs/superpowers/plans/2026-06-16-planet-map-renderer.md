# Planet Map Renderer Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Expose the Rust planet generation pipeline to Godot as a `PlanetMapRenderer` GodotClass that serves LOD-aware map textures for a 3D planet sphere.

**Architecture:** Extract all generation types and functions from `heatmap_export.rs` into a library module (`planet_gen.rs`), add a `PlanetParams::oros()` convenience constructor, then wrap the pipeline in a `PlanetMapRenderer` GodotClass that caches stable data and computes seasonal textures on demand via phasor sampling. Godot drives a tick counter and polls Rust at a configurable interval.

**Tech Stack:** Rust, gdext 0.5.3, Godot 4.6, GDScript

---

## File Map

| Action | Path | Responsibility |
|--------|------|----------------|
| Create | `src/planet_gen.rs` | All generation types and functions (moved from example) |
| Create | `src/planet_renderer.rs` | `PlanetMapRenderer` GodotClass |
| Create | `godot/scripts/planet_view.gd` | Tick counter, texture swap |
| Modify | `src/lib.rs` | Add module declarations |
| Modify | `examples/heatmap_export.rs` | Replace local types with library imports |
| Modify | `godot/node_3d.tscn` | Attach script, rebuild if needed |

---

## Task 1: Extract generation code into `src/planet_gen.rs`

**Files:**
- Create: `src/planet_gen.rs`
- Modify: `src/lib.rs`
- Modify: `examples/heatmap_export.rs`

- [ ] **Step 1: Create `src/planet_gen.rs`**

Copy every declaration from `examples/heatmap_export.rs` *except* `fn main()` into the new file. Replace the example's import block at the top with:

```rust
use crate::universe::{
    AtmosphereTag, CosmicCoordinates, EntityAge, Footprint, GeoTag,
    LiquidTag, Planet, Star, StarKind,
};
use crate::bio::{
    AtmosphereAffinity, AtmosphereRelationship, FoodTag, LifeBasis,
    ReproductionKind, ReproductionProfile, ReproductiveMethod, ReproductiveRole,
    RespirationMedium, SexKind, Solvent, Species, SpeciesKind, SpeciesSentience,
};
use crate::common::Range;
use uuid::Uuid;
use image::{ImageBuffer, Rgb};
use noise::{Fbm, NoiseFn, Perlin};
use std::cmp::Reverse;
use std::collections::{BinaryHeap, HashMap, VecDeque};
use std::f64::consts::PI;
```

- [ ] **Step 2: Add `pub` visibility throughout `planet_gen.rs`**

Add `pub` to every `struct`, `enum`, and free `fn`. For struct fields, mark these specifically:

- `HeatMap`: `pub width`, `pub height`, `pub data`
- `PlanetParams`: all fields `pub`
- `HydrologyResult`: all fields `pub`
- `PrecipPhasor`: `pub base`, `pub amp_x`, `pub amp_y`
- `SeasonalHydro`: `pub rainfall`, `pub snowmelt_amp`, `pub snowmelt_peak`, `pub flow_phasor`
- `Region`: all fields `pub`

Methods inside `impl` blocks get `pub` on each `fn` (not on the `impl` itself).

- [ ] **Step 3: Register the module in `src/lib.rs`**

```rust
pub mod planet_gen;
```

- [ ] **Step 4: Slim down `examples/heatmap_export.rs`**

Replace everything above `fn main()` in the example with:

```rust
use demiurge_rust::planet_gen::*;
use demiurge_rust::universe::{
    AtmosphereTag, CosmicCoordinates, EntityAge, Footprint, GeoTag,
    LiquidTag, Planet, Star, StarKind,
};
use demiurge_rust::bio::{
    Species, SpeciesKind, SpeciesSentience, AtmosphereAffinity,
    AtmosphereRelationship, FoodTag, LifeBasis, ReproductionKind,
    ReproductionProfile, ReproductiveMethod, ReproductiveRole,
    RespirationMedium, SexKind, Solvent,
};
use demiurge_rust::common::Range;
use uuid::Uuid;
use image::{ImageBuffer, Rgb};
use std::collections::HashMap;
use std::f64::consts::PI;
```

- [ ] **Step 5: Build and run**

```bash
cargo build
cargo run --example heatmap_export -- oros
```

Expected: compiles cleanly; `composite.png` generated as before.

- [ ] **Step 6: Commit**

```bash
git add src/planet_gen.rs src/lib.rs examples/heatmap_export.rs
git commit -m "Extract planet generation into src/planet_gen library module"
```

---

## Task 2: Add `PlanetParams::oros()` to `planet_gen.rs`

The Oros construction is currently inline in `main()`. Make it a reusable library method.

**Files:**
- Modify: `src/planet_gen.rs`
- Modify: `examples/heatmap_export.rs`

- [ ] **Step 1: Add `oros()` to the `impl PlanetParams` block in `planet_gen.rs`**

```rust
pub fn oros() -> Self {
    let null_age = EntityAge {
        formation_billions: Some(3), formation_millions: None,
        formation_thousands: None,   formation_years: 0,
        formation_month: 0,          formation_day: 0,
        age_billions: Some(3),       age_millions: None,
        age_thousands: None,         age_years: None,
        age_months: None,            age_days: 0,
    };
    let star = Star {
        id:            Uuid::nil(),
        name:          "Outer Reach Star".to_string(),
        age:           EntityAge {
            formation_billions: Some(4), formation_millions: None,
            formation_thousands: None,   formation_years: 0,
            formation_month: 0,          formation_day: 0,
            age_billions: Some(4),       age_millions: None,
            age_thousands: None,         age_years: None,
            age_months: None,            age_days: 0,
        },
        kind:          StarKind::YellowDwarf,
        luminosity:    1.08,
        parent_id:     None,
        companion_ids: None,
        domain_exp:    HashMap::new(),
    };
    let oros = Planet {
        id:              Uuid::parse_str("e3f92fd2-3501-40b4-957f-95d65dc4b51e").unwrap(),
        name:            "Oros".to_string(),
        age:             null_age,
        parent_id:       None,
        child_ids:       None,
        coord:           CosmicCoordinates { x: 1.3, y: 0.0, z: 0.0 },
        orbital_period:  536.0,
        axial_tilt:      22.0,
        rotation_period: 1.25,
        radius:          0.88,
        gravity:         0.83,
        base_press:      84.7,
        atmo: HashMap::from([
            (AtmosphereTag::WaterVapor,    0.08),
            (AtmosphereTag::Nitrogen,      0.76),
            (AtmosphereTag::Oxygen,        0.15),
            (AtmosphereTag::CarbonDioxide, 0.01),
        ]),
        geo: HashMap::from([
            (GeoTag::Silicate,    0.48),
            (GeoTag::Basaltic,    0.20),
            (GeoTag::Ferrous,     0.14),
            (GeoTag::Carbonate,   0.08),
            (GeoTag::Crystalline, 0.10),
        ]),
        volcanism:       0.20,
        hydro:           HashMap::from([(LiquidTag::Water, 1.0)]),
        liquid_coverage: 0.33,
        civ_ids:         None,
        species_ids:     None,
        domain_exp:      HashMap::new(),
        footprint:       Footprint { kind: HashMap::new() },
    };
    Self::from_planet(&oros, &star)
}
```

- [ ] **Step 2: Simplify the `"oros"` arm in `examples/heatmap_export.rs`**

Replace the verbose Oros construction block in `main()`:

```rust
"oros" => (PlanetParams::oros(), "Oros".to_string()),
```

- [ ] **Step 3: Build and run**

```bash
cargo build
cargo run --example heatmap_export -- oros
```

Expected: same output as before.

- [ ] **Step 4: Commit**

```bash
git add src/planet_gen.rs examples/heatmap_export.rs
git commit -m "Add PlanetParams::oros() convenience constructor"
```

---

## Task 3: Create `src/planet_renderer.rs`

**Files:**
- Create: `src/planet_renderer.rs`
- Modify: `src/lib.rs`

- [ ] **Step 1: Add the module to `src/lib.rs`**

```rust
mod planet_renderer;
```

- [ ] **Step 2: Create `src/planet_renderer.rs`**

```rust
use godot::prelude::*;
use crate::planet_gen::{
    HeatMap, PlanetParams, SeasonalHydro, PrecipPhasor,
    classify_snowpack, generate_seasonal_hydro, generate_seasonal_precip,
    generate_sea_ice, generate_glacier, flood_fill_ocean,
    compute_salt_flat_dist, render_composite_map, sample_precip_phasor,
};
use std::f64::consts::PI;

const WIDTH: usize = 512;
const HEIGHT: usize = 256;

#[derive(GodotClass)]
#[class(base=Node)]
pub struct PlanetMapRenderer {
    params:         Option<PlanetParams>,
    elevation:      Option<HeatMap>,
    temperature:    Option<HeatMap>,
    aridity:        Option<HeatMap>,
    is_ocean:       Option<Vec<bool>>,
    is_glacier:     Option<Vec<bool>>,
    is_sea_ice:     Option<Vec<bool>>,
    is_salt_flat:   Option<Vec<bool>>,
    salt_flat_dist: Option<Vec<u32>>,
    annual_hydro:   Option<HeatMap>,
    seasonal_hydro: Option<SeasonalHydro>,
    base: Base<Node>,
}

#[godot_api]
impl INode for PlanetMapRenderer {
    fn init(base: Base<Node>) -> Self {
        Self {
            params: None, elevation: None, temperature: None,
            aridity: None, is_ocean: None, is_glacier: None,
            is_sea_ice: None, is_salt_flat: None, salt_flat_dist: None,
            annual_hydro: None, seasonal_hydro: None,
            base,
        }
    }
}

#[godot_api]
impl PlanetMapRenderer {
    #[func]
    pub fn initialize(&mut self) {
        let params = PlanetParams::oros();
        let seed   = params.seed;

        let mut elevation = HeatMap::generate_elevation(WIDTH, HEIGHT, seed, params.warp_strength);
        elevation.roughen_coastline(params.sea_level, seed.wrapping_add(10));

        let is_ocean         = flood_fill_ocean(&elevation.data, WIDTH, HEIGHT, params.sea_level);
        let temperature      = HeatMap::generate_temperature(&elevation, &params);
        let is_sea_ice       = generate_sea_ice(&temperature, &is_ocean, params.sea_ice_temp_threshold);
        let rainfall_phasors = generate_seasonal_precip(
            &elevation, &is_ocean, &temperature, &is_sea_ice, &params,
        );
        let precipitation    = HeatMap {
            width: WIDTH, height: HEIGHT,
            data: rainfall_phasors.iter().map(|p| p.base as f64).collect(),
        };
        let is_glacier       = generate_glacier(&temperature, &is_ocean, params.glacier_temp_threshold);
        let aridity          = HeatMap::generate_aridity(&temperature, &precipitation, params.et_factor);
        let result           = HeatMap::generate_hydrology(
            &elevation, &is_ocean, &precipitation, &is_glacier, &params,
        );
        let snowmelt_amp     = classify_snowpack(&temperature, &is_ocean, &rainfall_phasors, &params);
        let seasonal_hydro   = generate_seasonal_hydro(
            rainfall_phasors, snowmelt_amp,
            (3.0 * PI / 2.0) as f32,
            &result.flow_to, &result.topo_order, &result.accumulation,
        );

        let salt_threshold = 0.35 + params.salt_flat_probability * 0.25;
        let is_salt_flat: Vec<bool> = (0..WIDTH * HEIGHT).map(|i| {
            result.is_endorheic[i] && !is_ocean[i] && !is_glacier[i]
                && aridity.data[i] < salt_threshold
        }).collect();
        let salt_flat_dist = compute_salt_flat_dist(&is_salt_flat, WIDTH, HEIGHT);

        self.annual_hydro   = Some(result.map);
        self.seasonal_hydro = Some(seasonal_hydro);
        self.params         = Some(params);
        self.elevation      = Some(elevation);
        self.temperature    = Some(temperature);
        self.aridity        = Some(aridity);
        self.is_ocean       = Some(is_ocean);
        self.is_glacier     = Some(is_glacier);
        self.is_sea_ice     = Some(is_sea_ice);
        self.is_salt_flat   = Some(is_salt_flat);
        self.salt_flat_dist = Some(salt_flat_dist);
    }

    #[func]
    pub fn annual_texture(&self) -> PackedByteArray {
        self.render_hydro(self.annual_hydro.as_ref().expect("call initialize() first"))
    }

    #[func]
    pub fn seasonal_texture(&self, phase: f64) -> PackedByteArray {
        let sh = self.seasonal_hydro.as_ref().expect("call initialize() first");
        let season_angle = phase * 2.0 * PI;
        let data: Vec<f64> = sh.flow_phasor.iter()
            .map(|p| sample_precip_phasor(p, season_angle) as f64)
            .collect();
        self.render_hydro(&HeatMap { width: WIDTH, height: HEIGHT, data })
    }

    #[func]
    pub fn update_interval_ticks(&self) -> i64 {
        // 1 tick = 1 Oros day; orbital period 536 days; update ~monthly
        536 / 12
    }

    fn render_hydro(&self, hydro_map: &HeatMap) -> PackedByteArray {
        let img = render_composite_map(
            WIDTH, HEIGHT, WIDTH * 3, HEIGHT * 3,
            hydro_map,
            self.elevation.as_ref().unwrap(),
            self.temperature.as_ref().unwrap(),
            self.aridity.as_ref().unwrap(),
            self.is_ocean.as_ref().unwrap(),
            self.is_glacier.as_ref().unwrap(),
            self.is_sea_ice.as_ref().unwrap(),
            self.is_salt_flat.as_ref().unwrap(),
            self.salt_flat_dist.as_ref().unwrap(),
            self.params.as_ref().unwrap(),
        );
        PackedByteArray::from(img.into_raw().as_slice())
    }
}
```

- [ ] **Step 3: Build**

```bash
cargo build
```

Expected: compiles cleanly. Fix any visibility errors by adding `pub` to the offending items in `planet_gen.rs`.

- [ ] **Step 4: Commit**

```bash
git add src/planet_renderer.rs src/lib.rs
git commit -m "Add PlanetMapRenderer GodotClass with seasonal texture pipeline"
```

---

## Task 4: Write the GDScript tick controller

**Files:**
- Create: `godot/scripts/planet_view.gd`

- [ ] **Step 1: Create `godot/scripts/planet_view.gd`**

```gdscript
extends Node3D

# Render resolution = WIDTH * 3 by HEIGHT * 3 (see planet_renderer.rs constants)
const TEX_WIDTH  := 512 * 3
const TEX_HEIGHT := 256 * 3
const ORBITAL_PERIOD := 536  # Oros days per year

var renderer: PlanetMapRenderer
var tick: int = 0
var update_interval: int

func _ready() -> void:
    renderer = PlanetMapRenderer.new()
    add_child(renderer)
    renderer.initialize()
    update_interval = renderer.update_interval_ticks()
    _apply_texture(renderer.annual_texture())

func _process(_delta: float) -> void:
    tick += 1
    if tick % update_interval == 0:
        var phase := float(tick % ORBITAL_PERIOD) / float(ORBITAL_PERIOD)
        _apply_texture(renderer.seasonal_texture(phase))

func _apply_texture(bytes: PackedByteArray) -> void:
    var img := Image.create_from_data(
        TEX_WIDTH, TEX_HEIGHT, false, Image.FORMAT_RGB8, bytes
    )
    var tex := ImageTexture.create_from_image(img)
    var mesh: MeshInstance3D = $MeshInstance3D
    var mat := mesh.get_surface_override_material(0)
    if mat == null:
        mat = StandardMaterial3D.new()
        mesh.set_surface_override_material(0, mat)
    (mat as StandardMaterial3D).albedo_texture = tex
```

- [ ] **Step 2: Commit**

```bash
git add godot/scripts/planet_view.gd
git commit -m "Add GDScript tick controller for planet texture updates"
```

---

## Task 5: Wire up the scene in Godot

**Files:**
- Modify: `godot/node_3d.tscn`

- [ ] **Step 1:** In the Godot editor, open `node_3d.tscn`. Select the root `Node3D` node and attach `scripts/planet_view.gd` to it.

- [ ] **Step 2:** Confirm the sphere mesh node is named exactly `MeshInstance3D` in the scene tree (the script references it as `$MeshInstance3D`). Rename in the scene tree if it differs.

- [ ] **Step 3:** Build the Rust crate, then run the project in Godot:

```bash
cargo build
```

Expected on startup: sphere displays the annual composite texture. Every 44 ticks, the texture updates to reflect the current season's hydrology. No errors in the Godot output panel.

- [ ] **Step 4: Commit**

```bash
git add godot/node_3d.tscn
git commit -m "Wire planet_view.gd to Node3D scene for live texture updates"
```
