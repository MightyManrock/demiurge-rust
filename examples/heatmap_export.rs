use demiurge_rust::universe::{
    AtmosphereTag, CosmicCoordinates, EntityAge, Footprint, GeoTag, LiquidTag, Planet, Star,
    StarKind,
};
use uuid::Uuid;
use image::{ImageBuffer, Rgb};
use noise::{Fbm, NoiseFn, Perlin};
use std::cmp::Reverse;
use std::collections::{BinaryHeap, HashMap, VecDeque};

// ── Data structures ──────────────────────────────────────────────────────────

/// A 2D float field over a normalized [0,1) x [0,1) coordinate plane.
/// x wraps (east-west); y clamps (poles do not connect).
struct HeatMap {
    width: usize,
    height: usize,
    data: Vec<f64>,
}

struct PlanetParams {
    seed:                   u32,
    // Elevation
    radius:                 f64, // Earth radii; scales feature sizes
    warp_strength:          f64,
    // Climate
    temp_baseline:          f64, // [0,1] equatorial warmth; from insolation + greenhouse
    temp_gradient:          f64, // how steeply temperature falls pole-ward; 1.0 = maximum
    lapse_factor:           f64, // elevation cooling rate
    precip_moisture:        f64, // [0,1] ocean evaporation ceiling; from WaterVapor fraction
    land_decay:             f64, // moisture loss per land cell
    slope_threshold:        f64, // min elevation gain to trigger rain shadow
    slope_loss:             f64, // rain shadow intensity
    base_arid:              f64, // minimum interior precipitation
    et_factor:              f64, // evapotranspiration scaling
    glacier_temp_threshold: f64, // temp below which land glaciates
    sea_ice_temp_threshold: f64, // temp below which ocean freezes
    sea_ice_evap_factor:    f64, // ocean evaporation reduction under sea ice
    // Hydrology
    sea_level:              f64, // from liquid_coverage
    max_lake_fill:          f64,
    aquifer_probability:    f64, // from Carbonate/Icy geo fraction
    river_threshold:        f64,
    glacier_melt_factor:    f64,
    // Region detection
    land_threshold:         f64,
    ocean_threshold:        f64,
    region_min_size:        usize,
    island_coast_dist:      usize,
    island_arch_dist:       usize,
    lon_weight:             f64,   // how much east-west spread from seed costs; 0 = no limit
}

impl PlanetParams {
    /// Earth-analog defaults — exactly reproduces the previous hardcoded behaviour.
    fn earth_like(seed: u32) -> Self {
        Self {
            seed,
            radius:                 1.0,
            warp_strength:          0.2,
            temp_baseline:          1.0,
            temp_gradient:          1.0,
            lapse_factor:           0.3,
            precip_moisture:        1.0,
            land_decay:             0.985,
            slope_threshold:        0.015,
            slope_loss:             0.5,
            base_arid:              0.05,
            et_factor:              0.35,
            glacier_temp_threshold: 0.20,
            sea_ice_temp_threshold: 0.14,
            sea_ice_evap_factor:    0.25,
            sea_level:              0.5,
            max_lake_fill:          0.04,
            aquifer_probability:    0.35,
            river_threshold:        400.0,
            glacier_melt_factor:    2.5,
            land_threshold:         0.15,
            ocean_threshold:        0.59,
            region_min_size:        150,
            island_coast_dist:      3,
            island_arch_dist:       25,
            lon_weight:             0.5,
        }
    }

    /// Derive generation parameters from a Planet entity and its host Star.
    fn from_planet(planet: &Planet, star: &Star) -> Self {
        // Insolation: luminosity / orbital-distance², normalised so that a
        // YellowDwarf at 1 AU gives exactly 1.0.
        let insolation =
            (star.luminosity as f64 / (planet.coord.x as f64).powi(2)).clamp(0.0, 3.0);

        // Greenhouse contribution from key atmospheric gases.  Fractions are
        // from the normalised atmo HashMap, so they already sum to ≤ 1.
        let co2 = *planet.atmo.get(&AtmosphereTag::CarbonDioxide).unwrap_or(&0.0) as f64;
        let ch4 = *planet.atmo.get(&AtmosphereTag::Methane).unwrap_or(&0.0) as f64;
        let h2o = *planet.atmo.get(&AtmosphereTag::WaterVapor).unwrap_or(&0.0) as f64;
        let nh3 = *planet.atmo.get(&AtmosphereTag::Ammonia).unwrap_or(&0.0) as f64;
        let greenhouse = co2 * 0.5 + ch4 * 0.8 + h2o * 0.2 + nh3 * 0.4;

        // Equatorial surface temperature: insolation shifted up by greenhouse.
        // Earth (insolation≈1, minimal greenhouse) → temp_baseline ≈ 1.0.
        let temp_baseline = (insolation + greenhouse).clamp(0.0, 1.0);

        // Axial tilt flattens the equator-to-pole temperature gradient.
        // tilt=0 → gradient=1.0 (maximum drop), tilt=90 → gradient=0.1.
        let tilt_norm = (planet.axial_tilt as f64 / 90.0).clamp(0.0, 1.0);
        let temp_gradient = 0.10 + 0.90 * (1.0 - tilt_norm);

        // Atmospheric moisture: WaterVapor fraction scaled by gravity (higher
        // gravity retains more atmosphere, supporting richer moisture cycles).
        let precip_moisture = (h2o * planet.gravity as f64).clamp(0.0, 1.0);
        let base_arid = (0.20 - precip_moisture * 0.15).max(0.01);

        // Ice thresholds shift with global temperature: colder planets freeze
        // at proportionally higher temperatures.
        let glacier_temp_threshold = (0.20 * (1.0 - temp_baseline * 0.5)).clamp(0.05, 0.40);
        let sea_ice_temp_threshold = glacier_temp_threshold * 0.70;

        // Carbonate and icy crusts favour subsurface water retention.
        let carbonate = *planet.geo.get(&GeoTag::Carbonate).unwrap_or(&0.0) as f64;
        let icy       = *planet.geo.get(&GeoTag::Icy).unwrap_or(&0.0) as f64;
        let aquifer_probability = (0.10 + carbonate * 0.60 + icy * 0.30).clamp(0.0, 1.0);

        // Larger, more volcanically active planets have rougher terrain.
        let warp_strength =
            (0.10 + planet.volcanism as f64 * 0.30) * (planet.radius as f64).sqrt();

        Self {
            seed:                   seed_from_uuid(*planet.id.as_bytes()),
            radius:                 planet.radius as f64,
            warp_strength,
            temp_baseline,
            temp_gradient,
            lapse_factor:           0.3,
            precip_moisture,
            land_decay:             0.985,
            slope_threshold:        0.015,
            slope_loss:             0.5,
            base_arid,
            et_factor:              0.35,
            glacier_temp_threshold,
            sea_ice_temp_threshold,
            sea_ice_evap_factor:    0.25,
            sea_level:              planet.liquid_coverage as f64,
            max_lake_fill:          0.04,
            aquifer_probability,
            river_threshold:        400.0,
            glacier_melt_factor:    2.5,
            land_threshold:         0.15,
            ocean_threshold:        0.59,
            region_min_size:        150,
            island_coast_dist:      3,
            island_arch_dist:       25,
            lon_weight:             0.5,
        }
    }
}

struct HydrologyResult {
    map: HeatMap,
    /// Cells where rivers sink underground rather than running dry.
    aquifer_zones: Vec<(usize, usize)>,
}

// ── HeatMap generation ───────────────────────────────────────────────────────

impl HeatMap {
    fn generate_elevation(width: usize, height: usize, seed: u32, warp_strength: f64) -> Self {
        let fbm = Fbm::<Perlin>::new(seed);
        // Two decorrelated FBM fields warp the sample coordinates before the
        // main noise is read. This breaks up annular saddle features that FBM
        // occasionally produces, which otherwise manifest as ring-shaped trenches
        // that fill with circuit rivers. Spatial offsets (5.2, 1.3) decorrelate
        // the two warp axes from each other and from the main field.
        let warp_a = Fbm::<Perlin>::new(seed.wrapping_add(1));
        let warp_b = Fbm::<Perlin>::new(seed.wrapping_add(2));
        let warp_c = Fbm::<Perlin>::new(seed.wrapping_add(3));
        // Radius so that the equatorial circumference equals 3.5 — preserves
        // feature frequency at the equator. All three FBM fields are sampled at
        // the 3D sphere-surface point, making the noise seamless in both x and y
        // and causing features to converge naturally at the poles.
        let r = 3.5 / std::f64::consts::TAU;

        let mut data = Vec::with_capacity(width * height);
        for y in 0..height {
            for x in 0..width {
                let lon = x as f64 / width as f64 * std::f64::consts::TAU;
                let lat = (y as f64 / height as f64 - 0.5) * std::f64::consts::PI;
                let cos_lat = lat.cos();
                let sx = r * cos_lat * lon.cos();
                let sy = r * cos_lat * lon.sin();
                let sz = r * lat.sin();

                // All three warp fields sampled at sphere-surface coords.
                let dx = warp_a.get([sx, sy, sz]) * warp_strength;
                let dy = warp_b.get([sx + 5.2, sy + 1.3, sz + 3.7]) * warp_strength;
                let dz = warp_c.get([sx + 2.8, sy + 4.6, sz + 1.9]) * warp_strength;
                data.push(fbm.get([sx + dx, sy + dy, sz + dz]));
            }
        }

        // Normalize to [0, 1] using actual min/max.
        let min = data.iter().cloned().fold(f64::INFINITY, f64::min);
        let max = data.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        let range = max - min;
        for v in &mut data {
            *v = (*v - min) / range;
        }

        let mut hm = HeatMap { width, height, data };
        hm.smooth_low_variance(6, 0.002, 0.25);
        hm
    }

    /// Iteratively blends cells toward their neighborhood mean, but only where
    /// local variance is below the threshold — i.e. flat plains and plateaus.
    /// High-variance areas (ridgelines, mountain peaks) are left untouched.
    fn smooth_low_variance(&mut self, passes: usize, variance_threshold: f64, blend: f64) {
        for _ in 0..passes {
            let prev = self.data.clone();
            for y in 0..self.height {
                for x in 0..self.width {
                    let idx = y * self.width + x;
                    let neighbors = neighbors_8(x, y, self.width, self.height);
                    let n = neighbors.len() as f64;
                    let vals: Vec<f64> =
                        neighbors.iter().map(|&(nx, ny)| prev[ny * self.width + nx]).collect();
                    let mean = vals.iter().sum::<f64>() / n;
                    let variance =
                        vals.iter().map(|&v| (v - mean).powi(2)).sum::<f64>() / n;
                    if variance < variance_threshold {
                        let neighborhood_mean = (mean * n + prev[idx]) / (n + 1.0);
                        self.data[idx] = prev[idx] * (1.0 - blend) + neighborhood_mean * blend;
                    }
                }
            }
        }
    }

    /// Adds high-frequency detail noise near sea level, producing jagged
    /// coastlines — cliffs, inlets, sea stacks — from cells just above or below
    /// the waterline being nudged across it. Uses spherical sampling so the
    /// detail is seamless. Must be called before flood_fill_ocean.
    fn roughen_coastline(&mut self, sea_level: f64, seed: u32) {
        let detail = Fbm::<Perlin>::new(seed);
        // 4× finer than the main terrain scale (3.5) for visible coastal detail.
        let r = 50.0 / std::f64::consts::TAU;
        const AMPLITUDE: f64 = 0.08;
        // Gaussian bandwidth: how far from sea level the effect reaches.
        const BANDWIDTH: f64 = 0.05;

        let width = self.width;
        let height = self.height;
        for y in 0..height {
            for x in 0..width {
                let idx = y * width + x;
                let elev = self.data[idx];
                let dist = elev - sea_level;
                let weight = (-(dist * dist) / (2.0 * BANDWIDTH * BANDWIDTH)).exp();
                if weight < 0.01 {
                    continue;
                }
                let lon = x as f64 / width as f64 * std::f64::consts::TAU;
                let lat = (y as f64 / height as f64 - 0.5) * std::f64::consts::PI;
                let cos_lat = lat.cos();
                let sx = r * cos_lat * lon.cos();
                let sy = r * cos_lat * lon.sin();
                let sz = r * lat.sin();
                let noise = detail.get([sx, sy, sz]);
                self.data[idx] = (elev + noise * AMPLITUDE * weight).clamp(0.0, 1.0);
            }
        }
    }

    /// Latitude cosine + elevation lapse rate, scaled by planet params.
    ///
    /// `temp_baseline` sets the equatorial surface temperature [0,1]; `temp_gradient`
    /// controls how steeply it drops toward the poles (1.0 = full drop to 0, 0.1 = nearly flat).
    fn generate_temperature(elevation: &HeatMap, params: &PlanetParams) -> HeatMap {
        let width  = elevation.width;
        let height = elevation.height;

        let data = (0..width * height)
            .map(|idx| {
                let y = idx / width;
                let abs_lat =
                    (y as f64 - height as f64 / 2.0).abs() / (height as f64 / 2.0);
                let lat_shape = (abs_lat * std::f64::consts::FRAC_PI_2).cos();
                let lat_temp  =
                    params.temp_baseline * (1.0 - params.temp_gradient * (1.0 - lat_shape));
                (lat_temp - elevation.data[idx] * params.lapse_factor).clamp(0.0, 1.0)
            })
            .collect();

        HeatMap { width, height, data }
    }

    /// Atmospheric band function + row-sweep moisture advection + rain shadow.
    ///
    /// Two moisture fields are accumulated via double-pass row sweeps (one for
    /// westerlies, one for easterlies) and blended by a latitude-dependent
    /// westerly weight. The double-pass handles the east-west seam: carry from
    /// the end of each row's first pass seeds the second pass, so moisture
    /// wraps around the globe correctly.
    fn generate_precipitation(elevation: &HeatMap, is_ocean: &[bool], temperature: &HeatMap, is_sea_ice: &[bool], params: &PlanetParams) -> HeatMap {
        let width = elevation.width;
        let height = elevation.height;
        let n = width * height;

        let mut moisture_west = vec![0.0f64; n];
        let mut moisture_east = vec![0.0f64; n];

        // Westerly sweep: wind from west, moisture moves east.
        // Scan x=0→width-1 twice; second pass starts with carry from the end of
        // the first, so x=0 correctly inherits moisture wrapping from x=width-1.
        for y in 0..height {
            let mut carry = 0.0f64;
            for pass_x in 0..(width * 2) {
                let x = pass_x % width;
                let idx = y * width + x;
                if is_ocean[idx] {
                    // Sea ice dramatically reduces evaporation; open ocean = full moisture.
                    carry = if is_sea_ice[idx] { params.sea_ice_evap_factor } else { params.precip_moisture };
                } else {
                    let upwind_x = (x + width - 1) % width;
                    let raw_gain =
                        elevation.data[idx] - elevation.data[y * width + upwind_x];
                    let elev_gain = (raw_gain - params.slope_threshold).max(0.0);
                    carry = (carry * params.land_decay - elev_gain * params.slope_loss).max(0.0);
                }
                if pass_x >= width {
                    moisture_west[idx] = carry;
                }
            }
        }

        // Easterly sweep: wind from east, moisture moves west.
        // Scan x=width-1→0 twice; second pass starts with carry from x=0
        // so x=width-1 correctly inherits moisture wrapping from x=0.
        for y in 0..height {
            let mut carry = 0.0f64;
            for pass_i in 0..(width * 2) {
                let x = (width - 1) - (pass_i % width);
                let idx = y * width + x;
                if is_ocean[idx] {
                    carry = if is_sea_ice[idx] { params.sea_ice_evap_factor } else { params.precip_moisture };
                } else {
                    let upwind_x = (x + 1) % width;
                    let raw_gain =
                        elevation.data[idx] - elevation.data[y * width + upwind_x];
                    let elev_gain = (raw_gain - params.slope_threshold).max(0.0);
                    carry = (carry * params.land_decay - elev_gain * params.slope_loss).max(0.0);
                }
                if pass_i >= width {
                    moisture_east[idx] = carry;
                }
            }
        }

        // Cold air holds less moisture: this dampens precipitation at high latitudes
        // and high altitudes independently of the circulation band factor.
        // Range: 0.3 (arctic) → 1.0 (tropical), so even the coldest cells get some snowfall.
        let data = (0..n)
            .map(|idx| {
                let y = idx / width;
                let abs_lat =
                    (y as f64 - height as f64 / 2.0).abs() / (height as f64 / 2.0);
                let w = westerly_weight(abs_lat);
                let moisture = moisture_west[idx] * w + moisture_east[idx] * (1.0 - w);
                let band = lat_band_factor(abs_lat);
                let moisture_capacity = (0.3 + 0.7 * temperature.data[idx]).clamp(0.3, 1.0);
                (band * (params.base_arid + moisture * (1.0 - params.base_arid)) * moisture_capacity).clamp(0.0, 1.0)
            })
            .collect();

        HeatMap { width, height, data }
    }

    fn generate_hydrology(
        elevation: &HeatMap,
        is_ocean: &[bool],
        precipitation: &HeatMap,
        is_glacier: &[bool],
        params: &PlanetParams,
    ) -> HydrologyResult {
        let width = elevation.width;
        let height = elevation.height;
        let n = width * height;

        // ── Phase 1: Priority-flood depression filling ───────────────────────
        //
        // Seeds the flood from all ocean cells and pole edges. Each cell's
        // filled elevation is raised to at least its natural elevation, ensuring
        // every land cell has a monotonic downhill path to the ocean. Without
        // this, FBM pits would trap flow accumulation and produce disconnected
        // puddles instead of rivers that reach the sea.
        //
        // Cells are processed lowest-first (min-heap) so fill propagates from
        // the ocean upward, never raising a cell above the height needed to
        // just reach its outlet.

        let mut filled = elevation.data.clone();
        let mut in_queue = vec![false; n];
        // (Reverse(key), index): min-heap by filled elevation.
        let mut heap: BinaryHeap<(Reverse<u64>, usize)> = BinaryHeap::new();

        for y in 0..height {
            for x in 0..width {
                let idx = y * width + x;
                let is_pole = y == 0 || y == height - 1;
                if is_pole || is_ocean[idx] {
                    in_queue[idx] = true;
                    heap.push((Reverse(float_key(filled[idx])), idx));
                }
            }
        }

        while let Some((_, idx)) = heap.pop() {
            let cx = idx % width;
            let cy = idx / width;
            for (nx, ny) in neighbors_8(cx, cy, width, height) {
                let nidx = ny * width + nx;
                if in_queue[nidx] {
                    continue;
                }
                in_queue[nidx] = true;
                filled[nidx] = f64::max(elevation.data[nidx], filled[idx]);
                heap.push((Reverse(float_key(filled[nidx])), nidx));
            }
        }

        // ── Phase 2: D8 flow directions on filled terrain ───────────────────
        //
        // Each land cell points to the neighbor with the steepest downhill
        // gradient. For neighbors inside filled (lake) regions, natural elevation
        // is used instead of filled elevation — otherwise the entire flat lake
        // surface has zero slope and D8 can't route water to the deep center,
        // leaving endorheic cells with no incoming accumulation.

        let is_lake_cell = |i: usize| filled[i] > elevation.data[i] + 1e-6;

        let mut flow_to: Vec<Option<usize>> = vec![None; n];

        for y in 0..height {
            for x in 0..width {
                let idx = y * width + x;
                if is_ocean[idx] {
                    continue;
                }
                let h = filled[idx];
                let mut best = None;
                let mut steepest = 0.0f64;

                for (nx, ny) in neighbors_8(x, y, width, height) {
                    let nidx = ny * width + nx;
                    // Within flat lake regions use natural elevation so flow
                    // routes toward the deep center rather than stalling.
                    let nh = if is_lake_cell(nidx) { elevation.data[nidx] } else { filled[nidx] };
                    // Correct dx for x-axis wrap.
                    let raw_dx = nx as i64 - x as i64;
                    let dx = if raw_dx.abs() > 1 { -raw_dx.signum() } else { raw_dx };
                    let dy = ny as i64 - y as i64;
                    let dist = ((dx * dx + dy * dy) as f64).sqrt();
                    let slope = (h - nh) / dist;
                    if slope > steepest {
                        steepest = slope;
                        best = Some(nidx);
                    }
                }
                flow_to[idx] = best;
            }
        }

        // ── Phase 3: Per-basin classification ──────────────────────────────
        //
        // BFS over connected components of lake cells (filled > natural elev).
        // Each basin is classified as a unit before outlet routing and flow
        // accumulation run, so both can use the correct endorheic flags.

        let mut basin_id: Vec<Option<usize>> = vec![None; n];
        let mut basins: Vec<Vec<usize>> = Vec::new();

        for start in 0..n {
            if !is_lake_cell(start) || basin_id[start].is_some() || is_ocean[start] {
                continue;
            }
            let id = basins.len();
            basins.push(Vec::new());
            let mut bfs = VecDeque::new();
            bfs.push_back(start);
            basin_id[start] = Some(id);
            while let Some(idx) = bfs.pop_front() {
                basins[id].push(idx);
                let cx = idx % width;
                let cy = idx / width;
                for (nx, ny) in neighbors_8(cx, cy, width, height) {
                    let nidx = ny * width + nx;
                    if is_lake_cell(nidx) && basin_id[nidx].is_none() && !is_ocean[nidx] {
                        basin_id[nidx] = Some(id);
                        bfs.push_back(nidx);
                    }
                }
            }
        }

        let basin_endorheic: Vec<bool> = basins
            .iter()
            .map(|cells| {
                cells
                    .iter()
                    .map(|&i| filled[i] - elevation.data[i])
                    .fold(0.0f64, f64::max)
                    > params.max_lake_fill
            })
            .collect();

        // ── Phase 4: Outlet routing ─────────────────────────────────────────
        //
        // For each non-endorheic basin, find the single lowest rim cell (the
        // natural spill point) and BFS outward from it through the lake, forcing
        // every lake cell's flow_to toward the outlet. Without this, multiple
        // rim cells at nearly equal elevation all act as outlets simultaneously,
        // producing diffuse shore seepage rather than one clean river exit.

        let mut outlet_visited = vec![false; n];

        for (id, cells) in basins.iter().enumerate() {
            if basin_endorheic[id] {
                continue;
            }

            // Find the non-lake, non-ocean neighbor with the lowest natural
            // elevation adjacent to any cell in this basin — the spill point.
            let mut outlet_lake_cell = usize::MAX;
            let mut rim_cell = usize::MAX;
            let mut best_rim_elev = f64::INFINITY;

            for &idx in cells {
                let cx = idx % width;
                let cy = idx / width;
                for (nx, ny) in neighbors_8(cx, cy, width, height) {
                    let nidx = ny * width + nx;
                    if !is_lake_cell(nidx) && !is_ocean[nidx] {
                        let e = elevation.data[nidx];
                        if e < best_rim_elev {
                            best_rim_elev = e;
                            outlet_lake_cell = idx;
                            rim_cell = nidx;
                        }
                    }
                }
            }

            if rim_cell == usize::MAX {
                continue;
            }

            // Route the outlet lake cell directly to the rim.
            flow_to[outlet_lake_cell] = Some(rim_cell);

            // BFS outward from the outlet lake cell; each reached cell flows
            // toward the cell it was reached from (i.e., toward the outlet).
            outlet_visited[outlet_lake_cell] = true;
            let mut bfs = VecDeque::new();
            bfs.push_back(outlet_lake_cell);
            while let Some(idx) = bfs.pop_front() {
                let cx = idx % width;
                let cy = idx / width;
                for (nx, ny) in neighbors_8(cx, cy, width, height) {
                    let nidx = ny * width + nx;
                    if basin_id[nidx] == Some(id) && !outlet_visited[nidx] {
                        outlet_visited[nidx] = true;
                        flow_to[nidx] = Some(idx);
                        bfs.push_back(nidx);
                    }
                }
            }

            // Reset visited flags using the known cell list (O(basin_size)).
            for &idx in cells {
                outlet_visited[idx] = false;
            }
        }

        // ── Phase 5: Flow accumulation ──────────────────────────────────────
        //
        // Process land cells highest-first. Each cell adds its precipitation-
        // weighted contribution to its downstream neighbor, now using the
        // outlet-corrected flow_to graph.

        let mut land_order: Vec<usize> = (0..n).filter(|&i| !is_ocean[i]).collect();
        land_order.sort_unstable_by(|&a, &b| filled[b].total_cmp(&filled[a]));

        let land_count = land_order.len();
        let mean_land_precip = if land_count > 0 {
            land_order.iter().map(|&i| precipitation.data[i]).sum::<f64>() / land_count as f64
        } else {
            1.0
        };
        // Glacier cells contribute extra flow representing meltwater. The bonus
        // is multiplicative so high-precip glaciers (wet snowfields) feed larger rivers.
        let mut accumulation: Vec<f64> = (0..n)
            .map(|i| {
                if is_ocean[i] { return 0.0; }
                let base = precipitation.data[i] / mean_land_precip;
                if is_glacier[i] { base * params.glacier_melt_factor } else { base }
            })
            .collect();
        for &idx in &land_order {
            if let Some(ds) = flow_to[idx] {
                accumulation[ds] += accumulation[idx];
            }
        }

        // ── Phase 6: Aquifer zone identification ────────────────────────────

        let endorheic: Vec<bool> = (0..n)
            .map(|i| match basin_id[i] {
                Some(id) => basin_endorheic[id],
                None => false,
            })
            .collect();

        let mut aquifer_zones = Vec::new();
        for idx in 0..n {
            if endorheic[idx] && accumulation[idx] >= params.river_threshold {
                if cell_hash(idx % width, idx / width) < params.aquifer_probability {
                    aquifer_zones.push((idx % width, idx / width));
                }
            }
        }

        // ── Phase 7: Encode into hydrology HeatMap ──────────────────────────
        //
        // Value ranges:
        //   0.0        = dry land (no water present)
        //   (0.0, 0.3] = river, proportional to log flow accumulation
        //   (0.3, 0.5] = lake, proportional to fill depth
        //   (0.5, 1.0] = ocean, proportional to depth below sea level

        let max_accum = accumulation.iter().cloned().fold(1.0f64, f64::max);
        let log_max = max_accum.ln().max(1.0);
        let mut data = vec![0.0f64; n];

        for y in 0..height {
            for x in 0..width {
                let idx = y * width + x;
                let elev = elevation.data[idx];
                let fill_depth = filled[idx] - elev;

                data[idx] = if is_ocean[idx] {
                    let depth = (params.sea_level - elev) / params.sea_level;
                    0.5 + depth.clamp(0.0, 1.0) * 0.5
                } else if endorheic[idx] {
                    0.0 // water disappears — dry basin floor
                } else if fill_depth > 1e-6 {
                    let depth_norm = (fill_depth / params.max_lake_fill).clamp(0.0, 1.0);
                    0.3 + depth_norm * 0.2
                } else if accumulation[idx] >= params.river_threshold {
                    let norm = accumulation[idx].ln() / log_max;
                    0.01 + norm * 0.29
                } else {
                    0.0
                };
            }
        }

        HydrologyResult {
            map: HeatMap { width, height, data },
            aquifer_zones,
        }
    }

    /// Effective moisture: precipitation minus potential evapotranspiration (which
    /// scales with temperature). Maps to [0, 1] where 0 = maximally arid and 1 =
    /// maximally humid. Used for biome classification and region detection.
    fn generate_aridity(temperature: &HeatMap, precipitation: &HeatMap, et_factor: f64) -> HeatMap {
        let data = temperature.data.iter().zip(precipitation.data.iter())
            .map(|(&t, &p)| ((p - t * et_factor + et_factor) / (1.0 + et_factor)).clamp(0.0, 1.0))
            .collect();
        HeatMap { width: temperature.width, height: temperature.height, data }
    }

    /// Sample by nearest-neighbor. x and y are in [0, 1).
    fn sample_nearest(&self, x: f64, y: f64) -> f64 {
        let px = (x.rem_euclid(1.0) * self.width as f64) as usize % self.width;
        let py = (y.clamp(0.0, 1.0 - f64::EPSILON) * self.height as f64) as usize;
        self.data[py * self.width + px]
    }

    /// Sample by bilinear interpolation. x and y are in [0, 1).
    fn sample(&self, x: f64, y: f64) -> f64 {
        let px = x.rem_euclid(1.0) * self.width as f64;
        let py = y.clamp(0.0, 1.0 - f64::EPSILON) * self.height as f64;

        let x0 = px.floor() as usize % self.width;
        let y0 = py.floor() as usize;
        let x1 = (x0 + 1) % self.width;
        let y1 = (y0 + 1).min(self.height - 1);

        let tx = px.fract();
        let ty = py.fract();

        let v00 = self.data[y0 * self.width + x0];
        let v10 = self.data[y0 * self.width + x1];
        let v01 = self.data[y1 * self.width + x0];
        let v11 = self.data[y1 * self.width + x1];

        let top = v00 + (v10 - v00) * tx;
        let bot = v01 + (v11 - v01) * tx;
        top + (bot - top) * ty
    }
}

// ── Climate helpers ──────────────────────────────────────────────────────────

/// Latitude precipitation factor based on Earth's general circulation bands.
/// Returns a [0, 1] multiplier applied before moisture weighting.
fn lat_band_factor(abs_lat: f64) -> f64 {
    // Piecewise linear through calibrated breakpoints:
    //   equator: 1.0 (ITCZ)
    //   ~30°:    0.2 (subtropical desert)
    //   ~50°:    0.6 (mid-lat cyclone belt)
    //   ~60°:    0.65 (mid-lat peak)
    //   ~70°:    0.3 (sub-polar)
    //   ~90°:    0.1 (polar desert)
    let stops: &[(f64, f64)] = &[
        (0.00, 1.00),
        (0.17, 0.90),
        (0.33, 0.20),
        (0.50, 0.60),
        (0.65, 0.65),
        (0.78, 0.30),
        (1.00, 0.10),
    ];
    for i in 0..stops.len() - 1 {
        let (ta, va) = stops[i];
        let (tb, vb) = stops[i + 1];
        if abs_lat <= tb {
            let t = (abs_lat - ta) / (tb - ta);
            return va + (vb - va) * t;
        }
    }
    stops.last().unwrap().1
}

/// Fraction of moisture contributed by the westerly sweep vs easterly sweep.
/// 1.0 = pure westerlies, 0.0 = pure easterlies.
fn westerly_weight(abs_lat: f64) -> f64 {
    // Westerlies dominate in mid-latitudes (~35–65°, abs_lat ~0.4–0.72).
    // Easterlies dominate in tropics and polar regions.
    let stops: &[(f64, f64)] = &[
        (0.00, 0.00),
        (0.25, 0.10),
        (0.40, 0.70),
        (0.55, 1.00),
        (0.70, 0.70),
        (0.78, 0.10),
        (1.00, 0.00),
    ];
    for i in 0..stops.len() - 1 {
        let (ta, va) = stops[i];
        let (tb, vb) = stops[i + 1];
        if abs_lat <= tb {
            let t = (abs_lat - ta) / (tb - ta);
            return va + (vb - va) * t;
        }
    }
    stops.last().unwrap().1
}

/// Returns a bool mask: true where a land cell is glaciated.
fn generate_glacier(temperature: &HeatMap, is_ocean: &[bool], threshold: f64) -> Vec<bool> {
    (0..temperature.data.len())
        .map(|i| !is_ocean[i] && temperature.data[i] < threshold)
        .collect()
}

/// Returns a bool mask: true where an ocean cell is frozen over as sea ice.
fn generate_sea_ice(temperature: &HeatMap, is_ocean: &[bool], threshold: f64) -> Vec<bool> {
    (0..temperature.data.len())
        .map(|i| is_ocean[i] && temperature.data[i] < threshold)
        .collect()
}

// ── Color functions ──────────────────────────────────────────────────────────

fn lerp_color(a: [u8; 3], b: [u8; 3], t: f64) -> [u8; 3] {
    [
        (a[0] as f64 + (b[0] as f64 - a[0] as f64) * t).round() as u8,
        (a[1] as f64 + (b[1] as f64 - a[1] as f64) * t).round() as u8,
        (a[2] as f64 + (b[2] as f64 - a[2] as f64) * t).round() as u8,
    ]
}

fn sample_gradient(t: f64, stops: &[([u8; 3], f64)]) -> [u8; 3] {
    for i in 0..stops.len() - 1 {
        let (ca, ta) = stops[i];
        let (cb, tb) = stops[i + 1];
        if t <= tb {
            let local_t = ((t - ta) / (tb - ta)).clamp(0.0, 1.0);
            return lerp_color(ca, cb, local_t);
        }
    }
    stops.last().unwrap().0
}

/// Raw elevation gradient: red (lowest) → yellow (mid) → green (highest).
fn elevation_color(t: f64) -> [u8; 3] {
    sample_gradient(
        t,
        &[
            ([255, 0, 0], 0.00),
            ([255, 255, 0], 0.50),
            ([0, 255, 0], 1.00),
        ],
    )
}

/// Terrain color for land in the composite. Accepts a pre-normalized land_t
/// in [0, 1] where 0 = coastline and 1 = highest peak.
fn terrain_color(land_t: f64) -> [u8; 3] {
    sample_gradient(
        land_t,
        &[
            ([220, 200, 150], 0.00), // coastal sand / beach
            ([180, 210, 120], 0.05), // lowland
            ([120, 175, 80], 0.20),  // plains / grassland
            ([80, 140, 60], 0.40),   // forest / hills
            ([110, 120, 70], 0.60),  // highland
            ([140, 110, 80], 0.75),  // rocky terrain
            ([170, 160, 150], 0.88), // grey rock
            ([230, 235, 240], 0.95), // snow line
            ([255, 255, 255], 1.00), // peak snow
        ],
    )
}

/// Water depth gradient covering rivers, lakes, and ocean.
///
/// Range mapping:
///   (0.0, 0.3] = rivers:  light cyan → medium blue
///   (0.3, 0.5] = lakes:   teal (distinct from ocean at the boundary)
///   (0.5, 1.0] = ocean:   medium blue → deep ocean
fn water_color(t: f64) -> [u8; 3] {
    sample_gradient(
        t,
        &[
            ([180, 230, 250], 0.01), // small stream
            ([80, 165, 220], 0.30),  // major river
            ([55, 175, 195], 0.30),  // lake edge (slight hue break from rivers)
            ([40, 130, 190], 0.50),  // deep lake / sea level
            ([30, 90, 180], 0.70),   // open ocean
            ([15, 40, 100], 1.00),   // deep ocean
        ],
    )
}

/// Temperature gradient: deep blue (coldest) → cyan → pale green → yellow → orange → deep red.
fn temperature_color(t: f64) -> [u8; 3] {
    sample_gradient(
        t,
        &[
            ([20, 20, 150], 0.00),   // arctic/polar
            ([70, 170, 230], 0.20),  // cold
            ([170, 220, 200], 0.40), // temperate cool
            ([240, 220, 130], 0.60), // warm
            ([230, 120, 30], 0.80),  // hot
            ([180, 20, 20], 1.00),   // extreme heat
        ],
    )
}

/// Precipitation gradient: tan (arid) → pale green → green → dark green → blue (extremely wet).
fn precipitation_color(t: f64) -> [u8; 3] {
    sample_gradient(
        t,
        &[
            ([210, 180, 130], 0.00), // hyperarid
            ([180, 200, 140], 0.20), // semi-arid
            ([100, 180, 100], 0.40), // moderate
            ([40, 130, 60], 0.60),   // wet
            ([20, 90, 130], 0.80),   // very wet
            ([10, 50, 180], 1.00),   // monsoon / extremely wet
        ],
    )
}

/// Sea ice color: flat white-grey, slightly more grey than land glacier to read
/// as a different surface. Dithers to ocean at its warm edge.
fn sea_ice_color(t: f64, threshold: f64) -> [u8; 3] {
    let normalized = (t / threshold).clamp(0.0, 1.0);
    sample_gradient(
        normalized,
        &[
            ([240, 245, 250], 0.00), // coldest — near-white
            ([195, 215, 230], 0.70), // mid — grey-blue
            ([160, 195, 220], 1.00), // warmest edge — more clearly blue-grey
        ],
    )
}

/// Glacier/ice color: pure white at coldest, pale blue at the warmer threshold edge.
/// t is the normalized temperature within the glaciated range [0, GLACIER_TEMP_THRESHOLD].
fn glacier_color(t: f64, threshold: f64) -> [u8; 3] {
    let normalized = (t / threshold).clamp(0.0, 1.0);
    sample_gradient(
        normalized,
        &[
            ([250, 253, 255], 0.00), // pure cold white
            ([200, 228, 248], 0.60), // pale ice blue
            ([175, 212, 240], 1.00), // warmer glacier edge
        ],
    )
}

/// Effective moisture gradient: orange-tan (arid) → pale green → teal (humid).
fn aridity_color(t: f64) -> [u8; 3] {
    sample_gradient(
        t,
        &[
            ([210, 150, 80],  0.00), // hyperarid
            ([220, 200, 130], 0.20), // arid
            ([190, 210, 150], 0.40), // semi-arid
            ([100, 175, 130], 0.60), // moderate
            ([50, 140, 120],  0.80), // humid
            ([20, 90, 110],   1.00), // very humid
        ],
    )
}

// ── Dithering ────────────────────────────────────────────────────────────────

const BAYER_4X4: [[f64; 4]; 4] = [
    [ 0.0 / 16.0,  8.0 / 16.0,  2.0 / 16.0, 10.0 / 16.0],
    [12.0 / 16.0,  4.0 / 16.0, 14.0 / 16.0,  6.0 / 16.0],
    [ 3.0 / 16.0, 11.0 / 16.0,  1.0 / 16.0,  9.0 / 16.0],
    [15.0 / 16.0,  7.0 / 16.0, 13.0 / 16.0,  5.0 / 16.0],
];

/// Returns true if the elevation value crosses a contour boundary between
/// this pixel and either of its right/down neighbors.
fn is_contour(e: f64, e_right: f64, e_down: f64, n_contours: usize) -> bool {
    let level = |v: f64| (v * n_contours as f64).floor() as i64;
    level(e) != level(e_right) || level(e) != level(e_down)
}

/// Ordered dither: quantize t to n_levels steps, using Bayer threshold at
/// render pixel (rx, ry) to break ties at level boundaries.
fn bayer_dither(t: f64, rx: usize, ry: usize, n_levels: usize) -> f64 {
    let threshold = BAYER_4X4[ry % 4][rx % 4];
    let scaled = t * n_levels as f64;
    let lo = scaled.floor();
    let level = if scaled - lo > threshold { lo + 1.0 } else { lo };
    (level / n_levels as f64).clamp(0.0, 1.0)
}

// ── Utilities ────────────────────────────────────────────────────────────────

/// BFS flood-fill from the global elevation minimum, marking all connected
/// cells below sea_level as ocean. Disconnected below-sea-level areas are
/// inland basins (not ocean) and fall through to normal lake/endorheic logic.
fn flood_fill_ocean(data: &[f64], width: usize, height: usize, sea_level: f64) -> Vec<bool> {
    let n = width * height;
    let mut is_ocean = vec![false; n];

    let min_idx = (0..n).min_by(|&a, &b| data[a].total_cmp(&data[b])).unwrap();
    if data[min_idx] >= sea_level {
        return is_ocean; // entirely dry planet
    }

    let mut queue = VecDeque::new();
    is_ocean[min_idx] = true;
    queue.push_back(min_idx);

    while let Some(idx) = queue.pop_front() {
        let x = idx % width;
        let y = idx / width;
        for (nx, ny) in neighbors_8(x, y, width, height) {
            let nidx = ny * width + nx;
            if !is_ocean[nidx] && data[nidx] < sea_level {
                is_ocean[nidx] = true;
                queue.push_back(nidx);
            }
        }
    }

    is_ocean
}

/// Convert a [0, 1] float to an integer heap key for min-heap ordering.
fn float_key(v: f64) -> u64 {
    (v.clamp(0.0, 1.0) * 1_000_000.0) as u64
}

/// 8-connected neighbors with full spherical topology.
///
/// x wraps east-west as normal. At the poles, going off the top or bottom edge
/// wraps to the same pole row but offset by width/2 — the equirectangular
/// projection of crossing the pole and emerging on the opposite side of the
/// planet. Duplicates are suppressed (can occur when multiple dx values map to
/// the same cell near the poles).
fn neighbors_8(x: usize, y: usize, width: usize, height: usize) -> Vec<(usize, usize)> {
    let mut result = Vec::with_capacity(8);
    for dy in -1i32..=1 {
        for dx in -1i32..=1 {
            if dx == 0 && dy == 0 {
                continue;
            }
            let mut nx = (x as i32 + dx).rem_euclid(width as i32) as usize;
            let ny_raw = y as i32 + dy;
            let ny = if ny_raw < 0 {
                // Crossed the north pole: emerge on the opposite side, same row.
                nx = (nx + width / 2) % width;
                0
            } else if ny_raw >= height as i32 {
                // Crossed the south pole: emerge on the opposite side, same row.
                nx = (nx + width / 2) % width;
                height - 1
            } else {
                ny_raw as usize
            };
            if !result.contains(&(nx, ny)) {
                result.push((nx, ny));
            }
        }
    }
    result
}

/// Fold a 128-bit UUID down to a u32 noise seed by XORing its four 32-bit
/// words. Uses all 128 bits so any change to the UUID changes the seed.
fn seed_from_uuid(bytes: [u8; 16]) -> u32 {
    let a = u32::from_le_bytes(bytes[0..4].try_into().unwrap());
    let b = u32::from_le_bytes(bytes[4..8].try_into().unwrap());
    let c = u32::from_le_bytes(bytes[8..12].try_into().unwrap());
    let d = u32::from_le_bytes(bytes[12..16].try_into().unwrap());
    a ^ b ^ c ^ d
}

/// Deterministic per-cell pseudo-random value in [0, 1).
/// Used to assign aquifer vs terminal outcome for endorheic basins.
fn cell_hash(x: usize, y: usize) -> f64 {
    let mut h = (x as u64)
        .wrapping_mul(2654435761)
        .wrapping_add((y as u64).wrapping_mul(2246822519));
    h ^= h >> 33;
    h = h.wrapping_mul(0xff51afd7ed558ccd);
    h ^= h >> 33;
    (h & 0xFFFF) as f64 / 65535.0
}

// ── Region detection ─────────────────────────────────────────────────────────

/// 4-connected neighbors for region detection. x wraps east-west; y clamps at poles.
fn neighbors_4(x: usize, y: usize, width: usize, height: usize) -> Vec<(usize, usize)> {
    let mut out = Vec::with_capacity(4);
    out.push(((x + width - 1) % width, y));
    out.push(((x + 1) % width, y));
    if y > 0 { out.push((x, y - 1)); }
    if y < height - 1 { out.push((x, y + 1)); }
    out
}

/// Aggregate description of a detected geographic region.
struct Region {
    id:                u32,
    size:              usize,
    mean_elev:         f64,
    mean_temp:         f64,
    mean_precip:       f64,
    mean_aridity:      f64,
    ocean_frac:        f64,
    glacier_frac:      f64,
    sea_ice_frac:      f64,
    island_components: usize, // 0 = continental, 1 = island, >1 = archipelago
}

impl Region {
    fn climate_character(&self) -> &'static str {
        if self.sea_ice_frac > 0.5   { return "Sea Ice"; }
        if self.glacier_frac > 0.5   { return "Glacier / Ice Sheet"; }
        if self.ocean_frac > 0.5 {
            if self.mean_temp < 0.20 { return "Polar Ocean"; }
            if self.mean_temp < 0.50 { return "Cold Ocean"; }
            if self.mean_temp < 0.70 { return "Temperate Ocean"; }
            return "Tropical Ocean";
        }
        if self.mean_temp < 0.20 {
            if self.mean_precip < 0.20 { return "Polar Desert"; }
            return "Tundra";
        }
        if self.mean_temp < 0.35 {
            if self.mean_precip < 0.20 { return "Cold Desert"; }
            return "Boreal Forest";
        }
        if self.mean_temp < 0.55 {
            if self.mean_precip < 0.15 { return "Temperate Desert"; }
            if self.mean_precip < 0.35 { return "Steppe"; }
            if self.mean_precip < 0.60 { return "Temperate Forest"; }
            return "Temperate Rainforest";
        }
        if self.mean_temp < 0.70 {
            if self.mean_precip < 0.20 { return "Hot Desert"; }
            if self.mean_precip < 0.45 { return "Mediterranean"; }
            return "Subtropical Forest";
        }
        if self.mean_precip < 0.20 { return "Tropical Desert"; }
        if self.mean_precip < 0.45 { return "Savanna"; }
        if self.mean_precip < 0.65 { return "Tropical Dry Forest"; }
        "Tropical Rainforest"
    }

    fn character(&self) -> String {
        let base = self.climate_character();
        match self.island_components {
            0 => base.to_string(),
            1 => format!("{base} Island"),
            _ => format!("{base} Archipelago"),
        }
    }
}

/// Cell classification for region detection.
#[derive(Clone, Copy, PartialEq)]
enum CellKind { Frozen, Ocean, Land }

fn cell_kind(idx: usize, is_ocean: &[bool], is_glacier: &[bool], is_sea_ice: &[bool]) -> CellKind {
    if is_sea_ice[idx] || is_glacier[idx] { CellKind::Frozen }
    else if is_ocean[idx]                  { CellKind::Ocean  }
    else                                   { CellKind::Land   }
}

/// Segment the map into geographic regions by multi-dimensional flood fill.
///
/// Three cell types are recognized: **Frozen** (sea ice or glacier), **Ocean**,
/// and **Land**. A BFS region may only expand into cells of its own type —
/// there is a hard barrier between open ocean and open land. Frozen cells form
/// their own pool and may freely mix sea ice and glaciated land.
///
/// `land_threshold` and `ocean_threshold` are the Euclidean similarity cutoffs
/// across (elevation, temperature, precipitation); ocean and frozen regions use
/// the laxer ocean threshold so the seas consolidate into fewer large regions.
///
/// Regions smaller than `min_size` cells are absorbed into their most-contacted
/// same-type neighbor.
///
/// Returns a flat region-ID map (one u32 per pixel) and a Vec<Region> sorted
/// largest-first.
fn detect_regions(
    elevation:       &HeatMap,
    temperature:     &HeatMap,
    precipitation:   &HeatMap,
    aridity:         &HeatMap,
    is_ocean:        &[bool],
    is_glacier:      &[bool],
    is_sea_ice:      &[bool],
    land_threshold:  f64,
    ocean_threshold: f64,
    min_size:        usize,
    coast_dist:      usize,
    arch_dist:       usize,
    lon_weight:      f64,
) -> (Vec<u32>, Vec<Region>) {
    let width  = elevation.width;
    let height = elevation.height;
    let n      = width * height;

    let mut region_map:   Vec<u32>       = vec![u32::MAX; n];
    let mut region_cells: Vec<Vec<usize>> = Vec::new();
    let mut region_kind:  Vec<CellKind>  = Vec::new();

    // Phase 1: BFS flood fill from each unvisited seed.
    // Similarity is checked against the SEED cell, not the frontier cell, to
    // prevent regions from drifting across gradual transitions.
    // Expansion is blocked across cell-kind boundaries (ocean ↔ land).
    for start in 0..n {
        if region_map[start] != u32::MAX { continue; }
        let id   = region_cells.len() as u32;
        let kind = cell_kind(start, is_ocean, is_glacier, is_sea_ice);
        let thr  = if kind == CellKind::Land { land_threshold } else { ocean_threshold };
        let mut cells = Vec::new();
        let mut queue = VecDeque::new();
        queue.push_back(start);
        region_map[start] = id;

        let se = elevation.data[start];
        let st = temperature.data[start];
        let sp = precipitation.data[start];
        let sx = start % width;

        while let Some(idx) = queue.pop_front() {
            cells.push(idx);
            let x = idx % width;
            let y = idx / width;
            for (nx, ny) in neighbors_4(x, y, width, height) {
                let nidx = ny * width + nx;
                if region_map[nidx] != u32::MAX { continue; }
                if cell_kind(nidx, is_ocean, is_glacier, is_sea_ice) != kind { continue; }
                let de  = se - elevation.data[nidx];
                let dt  = st - temperature.data[nidx];
                let dp  = sp - precipitation.data[nidx];
                // Wrap-aware longitude distance from seed, normalised to [0, 0.5].
                let raw_dx = (sx as i64 - nx as i64).unsigned_abs() as usize;
                let ddx = raw_dx.min(width - raw_dx) as f64 / width as f64;
                let dl  = lon_weight * ddx;
                if (de * de + dt * dt + dp * dp + dl * dl).sqrt() <= thr {
                    region_map[nidx] = id;
                    queue.push_back(nidx);
                }
            }
        }
        region_cells.push(cells);
        region_kind.push(kind);
    }

    // Phase 2: Absorb regions below min_size into their most-contacted neighbor,
    // processing smallest-first so orphan slivers merge before their targets do.
    loop {
        let small = region_cells.iter().enumerate()
            .filter(|(_, c)| !c.is_empty() && c.len() < min_size)
            .min_by_key(|(_, c)| c.len())
            .map(|(i, _)| i);
        let Some(sid) = small else { break };

        let mut counts: HashMap<u32, usize> = HashMap::new();
        let skind = region_kind[sid];
        for &idx in &region_cells[sid] {
            let x = idx % width;
            let y = idx / width;
            for (nx, ny) in neighbors_4(x, y, width, height) {
                let nid = region_map[ny * width + nx];
                if nid != sid as u32 && region_kind[nid as usize] == skind {
                    *counts.entry(nid).or_default() += 1;
                }
            }
        }
        if let Some((&target, _)) = counts.iter().max_by_key(|&(_, &c)| c) {
            let cells = std::mem::take(&mut region_cells[sid]);
            for &idx in &cells { region_map[idx] = target; }
            region_cells[target as usize].extend(cells);
        } else {
            region_cells[sid].clear(); // isolated — discard
        }
    }

    // Phase 2.5: Island detection pass.
    // Land cells discarded by min_size merging are either absorbed into a nearby
    // continental region (within COAST_DIST ocean hops) or grouped into standalone
    // island / archipelago regions via ocean BFS (within ARCH_DIST ocean hops).
    let island_coast_dist = coast_dist;
    let island_arch_dist  = arch_dist;

    // Per-region island-component count (0 = continental).  New island regions are
    // appended to region_cells below; their counts are pushed in lock-step.
    let mut island_parts: Vec<usize> = vec![0; region_cells.len()];

    // Snapshot which cells are free land: their Phase-1 region was discarded in Phase 2.
    // Computed before any region_map mutations so absorbed cells are detectable later.
    let free_land: Vec<bool> = (0..n).map(|idx| {
        let rid = region_map[idx] as usize;
        region_cells[rid].is_empty() && region_kind[rid] == CellKind::Land
    }).collect();

    // Group free-land cells by original region ID (each ID = one connected component).
    let mut comp_map: HashMap<u32, Vec<usize>> = HashMap::new();
    for idx in 0..n {
        if free_land[idx] { comp_map.entry(region_map[idx]).or_default().push(idx); }
    }
    let island_components: Vec<(u32, Vec<usize>)> = comp_map.into_iter().collect();
    let n_comps = island_components.len();

    // Step B+C: BFS from each component outward through ocean/frozen cells.
    // If a continental land region is reachable within COAST_DIST hops, absorb there.
    let mut coast_targets: Vec<Option<u32>> = vec![None; n_comps];
    for (ii, (_, icells)) in island_components.iter().enumerate() {
        let mut dist: Vec<u8> = vec![u8::MAX; n];
        let mut queue = VecDeque::new();
        for &idx in icells { dist[idx] = 0; queue.push_back(idx); }
        'coast: while let Some(idx) = queue.pop_front() {
            let x = idx % width;
            let y = idx / width;
            for (nx, ny) in neighbors_4(x, y, width, height) {
                let nidx = ny * width + nx;
                if dist[nidx] != u8::MAX { continue; }
                // Non-free, non-ocean, non-frozen → must be a continental land cell.
                if !free_land[nidx] && !is_ocean[nidx] && !is_glacier[nidx] && !is_sea_ice[nidx] {
                    coast_targets[ii] = Some(region_map[nidx]);
                    break 'coast;
                }
                let nd = dist[idx].saturating_add(1);
                if (is_ocean[nidx] || is_glacier[nidx] || is_sea_ice[nidx])
                    && nd <= island_coast_dist as u8
                {
                    dist[nidx] = nd;
                    queue.push_back(nidx);
                }
            }
        }
    }

    // Apply coastal absorptions.
    for (ii, (_, icells)) in island_components.iter().enumerate() {
        if let Some(target) = coast_targets[ii] {
            for &idx in icells { region_map[idx] = target; }
            region_cells[target as usize].extend_from_slice(icells);
        }
    }

    // Step D: Group remaining (non-absorbed) components into islands/archipelagos.
    // BFS expands through ocean/frozen cells; reaching another remaining component
    // within ARCH_DIST hops merges it into the current group.
    let remaining: Vec<usize> = (0..n_comps).filter(|&ii| coast_targets[ii].is_none()).collect();
    let n_remaining = remaining.len();
    if n_remaining > 0 {
        // Map original region ID → index in remaining[].
        let rid_to_ri: HashMap<u32, usize> = remaining.iter().enumerate()
            .map(|(ri, &ci)| (island_components[ci].0, ri))
            .collect();

        let mut group_of: Vec<Option<usize>> = vec![None; n_remaining];
        let mut groups: Vec<Vec<usize>> = Vec::new();

        for start_ri in 0..n_remaining {
            if group_of[start_ri].is_some() { continue; }
            let gid = groups.len();
            groups.push(vec![start_ri]);
            group_of[start_ri] = Some(gid);

            let mut visited = vec![false; n];
            let mut queue: VecDeque<(usize, u8)> = VecDeque::new();
            for &idx in &island_components[remaining[start_ri]].1 {
                visited[idx] = true;
                queue.push_back((idx, 0));
            }

            while let Some((idx, d)) = queue.pop_front() {
                let x = idx % width;
                let y = idx / width;
                for (nx, ny) in neighbors_4(x, y, width, height) {
                    let nidx = ny * width + nx;
                    if visited[nidx] { continue; }
                    visited[nidx] = true;
                    if free_land[nidx] {
                        // free_land is a snapshot: absorbed cells may now have a
                        // continental region_map entry not in rid_to_ri — skip them.
                        if let Some(&ri) = rid_to_ri.get(&region_map[nidx]) {
                            if group_of[ri].is_none() {
                                group_of[ri] = Some(gid);
                                groups[gid].push(ri);
                                for &cidx in &island_components[remaining[ri]].1 {
                                    if !visited[cidx] {
                                        visited[cidx] = true;
                                        queue.push_back((cidx, 0));
                                    }
                                }
                            }
                        }
                        continue;
                    }
                    let nd = d.saturating_add(1);
                    if (is_ocean[nidx] || is_glacier[nidx] || is_sea_ice[nidx])
                        && nd <= island_arch_dist as u8
                    {
                        queue.push_back((nidx, nd));
                    }
                }
            }
        }

        // Step E: Assign new region IDs for each island group.
        for group in &groups {
            let new_rid = region_cells.len() as u32;
            let n_parts = group.len();
            let mut all_cells = Vec::new();
            for &ri in group {
                let (_, cells) = &island_components[remaining[ri]];
                for &idx in cells { region_map[idx] = new_rid; }
                all_cells.extend_from_slice(cells);
            }
            region_cells.push(all_cells);
            region_kind.push(CellKind::Land);
            island_parts.push(n_parts);
        }
    }

    // Phase 3: Compact IDs and build Region structs.
    let mut new_id = 0u32;
    let mut id_remap: Vec<u32> = vec![u32::MAX; region_cells.len()];
    for (i, cells) in region_cells.iter().enumerate() {
        if !cells.is_empty() { id_remap[i] = new_id; new_id += 1; }
    }
    for v in region_map.iter_mut() {
        if *v != u32::MAX { *v = id_remap[*v as usize]; }
    }

    let mut regions: Vec<Region> = region_cells.iter().enumerate()
        .filter(|(_, c)| !c.is_empty())
        .map(|(old_id, cells)| {
            let sf = cells.len() as f64;
            Region {
                id:                id_remap[old_id],
                size:              cells.len(),
                mean_elev:         cells.iter().map(|&i| elevation.data[i]).sum::<f64>()     / sf,
                mean_temp:         cells.iter().map(|&i| temperature.data[i]).sum::<f64>()   / sf,
                mean_precip:       cells.iter().map(|&i| precipitation.data[i]).sum::<f64>() / sf,
                mean_aridity:      cells.iter().map(|&i| aridity.data[i]).sum::<f64>()       / sf,
                ocean_frac:        cells.iter().filter(|&&i| is_ocean[i]).count()    as f64 / sf,
                glacier_frac:      cells.iter().filter(|&&i| is_glacier[i]).count()  as f64 / sf,
                sea_ice_frac:      cells.iter().filter(|&&i| is_sea_ice[i]).count()  as f64 / sf,
                island_components: island_parts[old_id],
            }
        })
        .collect();

    regions.sort_unstable_by_key(|r| r.id);
    (region_map, regions)
}

// ── Political map helpers ─────────────────────────────────────────────────────

fn hsv_to_rgb(h: f64, s: f64, v: f64) -> [u8; 3] {
    let h6 = h * 6.0;
    let i  = h6.floor() as u32;
    let f  = h6 - h6.floor();
    let p  = v * (1.0 - s);
    let q  = v * (1.0 - s * f);
    let t  = v * (1.0 - s * (1.0 - f));
    let (r, g, b) = match i % 6 {
        0 => (v, t, p), 1 => (q, v, p), 2 => (p, v, t),
        3 => (p, q, v), 4 => (t, p, v), _ => (v, p, q),
    };
    [(r * 255.0) as u8, (g * 255.0) as u8, (b * 255.0) as u8]
}

/// Golden-ratio hue spacing with 4-tier S/V interleaving so that adjacent
/// region IDs differ in both hue and brightness.
fn political_color(region_id: u32) -> [u8; 3] {
    let hue = (region_id as f64 * 0.618_033_988_749_895) % 1.0;
    let (s, v) = match region_id % 4 {
        0 => (0.70, 0.88),
        1 => (0.88, 0.65),
        2 => (0.50, 0.93),
        _ => (0.82, 0.75),
    };
    hsv_to_rgb(hue, s, v)
}

/// Renders ASCII text onto an image using the 8×8 bitmap font at the given
/// pixel scale. Draws a 1-pixel black shadow first for legibility on any
/// background.
fn draw_text(
    img:   &mut ImageBuffer<Rgb<u8>, Vec<u8>>,
    x0:    i32,
    y0:    i32,
    text:  &str,
    color: [u8; 3],
    scale: i32,
) {
    use font8x8::UnicodeFonts;
    let iw = img.width()  as i32;
    let ih = img.height() as i32;
    for (ci, ch) in text.chars().enumerate() {
        let Some(glyph) = font8x8::BASIC_FONTS.get(ch) else { continue };
        let cx = x0 + ci as i32 * 8 * scale;
        for (row, &bits) in glyph.iter().enumerate() {
            for col in 0..8i32 {
                if bits & (1 << col) == 0 { continue; }
                for sy in 0..scale {
                    for sx in 0..scale {
                        let px = cx + col * scale + sx;
                        let py = y0 + row as i32 * scale + sy;
                        if px >= 0 && py >= 0 && px < iw && py < ih {
                            img.put_pixel(px as u32, py as u32, Rgb(color));
                        }
                    }
                }
            }
        }
    }
}

// ── Main ─────────────────────────────────────────────────────────────────────

fn main() {
    let width = 1024usize;
    let height = 512usize;

    // ── Oros ─────────────────────────────────────────────────────────────────
    let null_age = EntityAge {
        formation_billions: Some(3), formation_millions: None,
        formation_thousands: None,  formation_years: 0,
        formation_month: 0,         formation_day: 0,
        age_billions: Some(3),      age_millions: None,
        age_thousands: None,        age_years: None,
        age_months: None,           age_days: 0,
    };
    let star = Star {
        id:            Uuid::nil(),
        name:          "Outer Reach Star".to_string(),
        age:           EntityAge { formation_billions: Some(4), formation_millions: None,
                                   formation_thousands: None, formation_years: 0,
                                   formation_month: 0, formation_day: 0,
                                   age_billions: Some(4), age_millions: None,
                                   age_thousands: None, age_years: None,
                                   age_months: None, age_days: 0 },
        kind:          StarKind::YellowDwarf,
        luminosity:    1.08,
        parent_id:     None,
        companion_ids: None,
        domain_exp:    HashMap::new(),
    };
    let oros = Planet {
        id:             Uuid::parse_str("e3f92fd2-3501-40b4-957f-95d65dc4b51e").unwrap(),
        name:           "Oros".to_string(),
        age:            null_age,
        parent_id:      None,
        child_ids:      None,
        coord:          CosmicCoordinates { x: 1.3, y: 0.0, z: 0.0 },
        radius:         0.88,
        gravity:        0.83,
        axial_tilt:     22.0,
        atmo:           HashMap::from([
            (AtmosphereTag::WaterVapor,    0.08),
            (AtmosphereTag::Nitrogen,      0.76),
            (AtmosphereTag::Oxygen,        0.15),
            (AtmosphereTag::CarbonDioxide, 0.01),
        ]),
        geo:            HashMap::from([
            (GeoTag::Silicate,    0.48),
            (GeoTag::Basaltic,    0.20),
            (GeoTag::Ferrous,     0.14),
            (GeoTag::Carbonate,   0.08),
            (GeoTag::Crystalline, 0.10),
        ]),
        volcanism:      0.20,
        hydro:          HashMap::from([(LiquidTag::Water, 1.0)]),
        liquid_coverage: 0.33,
        civ_ids:        None,
        species_ids:    None,
        domain_exp:     HashMap::new(),
        footprint:      Footprint { kind: HashMap::new() },
    };
    let params = PlanetParams::from_planet(&oros, &star);
    let seed = params.seed;

    println!("Planet: {} | temp_baseline={:.2} temp_gradient={:.2} precip_moisture={:.3} sea_level={:.2}",
        oros.name, params.temp_baseline, params.temp_gradient, params.precip_moisture, params.sea_level);

    const RENDER_SCALE: usize = 3;
    const N_DITHER_LEVELS: usize = 16;
    let render_width = width * RENDER_SCALE;
    let render_height = height * RENDER_SCALE;

    println!("Generating {}x{} elevation map (seed {})...", width, height, seed);
    let mut elevation = HeatMap::generate_elevation(width, height, seed, params.warp_strength);
    elevation.roughen_coastline(params.sea_level, seed.wrapping_add(10));

    let elev_img = ImageBuffer::from_fn(width as u32, height as u32, |x, y| {
        let nx = x as f64 / width as f64;
        let ny = y as f64 / height as f64;
        Rgb(elevation_color(elevation.sample(nx, ny)))
    });
    elev_img.save("elevation.png").expect("failed to save elevation.png");
    println!("Saved elevation.png");

    let height_img = ImageBuffer::from_fn(width as u32, height as u32, |x, y| {
        let v = (elevation.sample(x as f64 / width as f64, y as f64 / height as f64) * 255.0)
            .round() as u8;
        Rgb([v, v, v])
    });
    height_img.save("heightmap.png").expect("failed to save heightmap.png");
    println!("Saved heightmap.png");

    // Ocean classification is shared by climate and hydrology.
    let is_ocean = flood_fill_ocean(&elevation.data, width, height, params.sea_level);

    println!("Generating climate...");
    let temperature   = HeatMap::generate_temperature(&elevation, &params);
    let is_sea_ice    = generate_sea_ice(&temperature, &is_ocean, params.sea_ice_temp_threshold);
    let precipitation = HeatMap::generate_precipitation(&elevation, &is_ocean, &temperature, &is_sea_ice, &params);
    let is_glacier    = generate_glacier(&temperature, &is_ocean, params.glacier_temp_threshold);
    let aridity       = HeatMap::generate_aridity(&temperature, &precipitation, params.et_factor);

    let temp_img = ImageBuffer::from_fn(width as u32, height as u32, |x, y| {
        let nx = x as f64 / width as f64;
        let ny = y as f64 / height as f64;
        Rgb(temperature_color(temperature.sample(nx, ny)))
    });
    temp_img.save("temperature.png").expect("failed to save temperature.png");
    println!("Saved temperature.png");

    let precip_img = ImageBuffer::from_fn(width as u32, height as u32, |x, y| {
        let nx = x as f64 / width as f64;
        let ny = y as f64 / height as f64;
        Rgb(precipitation_color(precipitation.sample(nx, ny)))
    });
    precip_img.save("precipitation.png").expect("failed to save precipitation.png");
    println!("Saved precipitation.png");

    let aridity_img = ImageBuffer::from_fn(width as u32, height as u32, |x, y| {
        let nx = x as f64 / width as f64;
        let ny = y as f64 / height as f64;
        Rgb(aridity_color(aridity.sample(nx, ny)))
    });
    aridity_img.save("aridity.png").expect("failed to save aridity.png");
    println!("Saved aridity.png");

    let glacier_img = ImageBuffer::from_fn(width as u32, height as u32, |x, y| {
        let idx = y as usize * width + x as usize;
        let nx = x as f64 / width as f64;
        let ny = y as f64 / height as f64;
        if is_glacier[idx] {
            Rgb(glacier_color(temperature.sample(nx, ny), params.glacier_temp_threshold))
        } else {
            Rgb([0u8, 0, 0])
        }
    });
    glacier_img.save("glacier.png").expect("failed to save glacier.png");
    println!("Saved glacier.png");

    let sea_ice_img = ImageBuffer::from_fn(width as u32, height as u32, |x, y| {
        let idx = y as usize * width + x as usize;
        let nx = x as f64 / width as f64;
        let ny = y as f64 / height as f64;
        if is_sea_ice[idx] {
            Rgb(sea_ice_color(temperature.sample(nx, ny), params.sea_ice_temp_threshold))
        } else {
            Rgb([0u8, 0, 0])
        }
    });
    sea_ice_img.save("sea_ice.png").expect("failed to save sea_ice.png");
    println!("Saved sea_ice.png");

    println!("Generating hydrology...");
    let result = HeatMap::generate_hydrology(&elevation, &is_ocean, &precipitation, &is_glacier, &params);
    println!(
        "  {} aquifer recharge zones identified",
        result.aquifer_zones.len()
    );

    // Raw hydrology: black for dry land, water gradient for wet cells.
    let hydro_img = ImageBuffer::from_fn(width as u32, height as u32, |x, y| {
        let nx = x as f64 / width as f64;
        let ny = y as f64 / height as f64;
        let hydro = result.map.sample(nx, ny);
        let color = if hydro > 0.0 { water_color(hydro) } else { [0, 0, 0] };
        Rgb(color)
    });
    hydro_img.save("hydrology.png").expect("failed to save hydrology.png");
    println!("Saved hydrology.png");

    // Composite: 3× render resolution with Bayer ordered dithering and
    // topographic contour lines. Contours are detected by checking whether
    // adjacent render pixels straddle an elevation level boundary.
    const N_CONTOURS: usize = 40;
    const CONTOUR_DARKEN: f64 = 0.90;
    const CONTOUR_DARKEN_WATER: f64 = 0.95;
    println!("Rendering composite at {}x{}...", render_width, render_height);
    let composite = ImageBuffer::from_fn(render_width as u32, render_height as u32, |rx, ry| {
        let nx = rx as f64 / render_width as f64;
        let ny = ry as f64 / render_height as f64;
        // Rivers (hydro <= 0.3) render as hard nearest-neighbor pixels — they're
        // often only one data pixel wide and bilinear dithering erases them.
        // Lakes and ocean get Bayer-dithered edges: for each of the four cardinal
        // directions, if the neighboring data pixel is dry land we lower the
        // coverage threshold so Bayer dithering converts some edge render pixels
        // to land. Checking all four directions gives symmetric dithering on every
        // side of the lake, not just right/bottom as bilinear interpolation would.
        let hydro_nearest = result.map.sample_nearest(nx, ny);
        let is_water = if hydro_nearest <= 0.0 {
            false
        } else if hydro_nearest <= 0.3 {
            true // rivers: hard pixel, no boundary dithering
        } else {
            const EDGE_COVERAGE: f64 = 0.0025;
            let dx = rx as usize / RENDER_SCALE;
            let dy = ry as usize / RENDER_SCALE;
            let off_x = rx as usize % RENDER_SCALE;
            let off_y = ry as usize % RENDER_SCALE;
            let neighbor = |ndx: i64, ndy: i64| -> f64 {
                let nnx = ndx.rem_euclid(width as i64) as usize;
                let nny = ndy.clamp(0, height as i64 - 1) as usize;
                result.map.data[nny * width + nnx]
            };
            let mut coverage = 1.0f64;
            if off_x == 0 && neighbor(dx as i64 - 1, dy as i64) <= 0.0 { coverage = EDGE_COVERAGE; }
            if off_x == 2 && neighbor(dx as i64 + 1, dy as i64) <= 0.0 { coverage = EDGE_COVERAGE; }
            if off_y == 0 && neighbor(dx as i64, dy as i64 - 1) <= 0.0 { coverage = EDGE_COVERAGE; }
            if off_y == 2 && neighbor(dx as i64, dy as i64 + 1) <= 0.0 { coverage = EDGE_COVERAGE; }
            BAYER_4X4[ry as usize % 4][rx as usize % 4] < coverage
        };
        let data_idx = (ry as usize / RENDER_SCALE) * width + (rx as usize / RENDER_SCALE);
        let dx = rx as usize / RENDER_SCALE;
        let dy = ry as usize / RENDER_SCALE;
        let off_x = rx as usize % RENDER_SCALE;
        let off_y = ry as usize % RENDER_SCALE;
        let mut color = if is_water && is_sea_ice[data_idx] {
            // Sea ice: dither at the warm boundary toward open ocean.
            let sea_ice_neighbor = |ndx: i64, ndy: i64| -> bool {
                let nnx = ndx.rem_euclid(width as i64) as usize;
                let nny = ndy.clamp(0, height as i64 - 1) as usize;
                is_sea_ice[nny * width + nnx]
            };
            const SEA_ICE_EDGE: f64 = 0.05;
            let mut coverage = 1.0f64;
            if off_x == 0 && !sea_ice_neighbor(dx as i64 - 1, dy as i64) { coverage = SEA_ICE_EDGE; }
            if off_x == 2 && !sea_ice_neighbor(dx as i64 + 1, dy as i64) { coverage = SEA_ICE_EDGE; }
            if off_y == 0 && !sea_ice_neighbor(dx as i64, dy as i64 - 1) { coverage = SEA_ICE_EDGE; }
            if off_y == 2 && !sea_ice_neighbor(dx as i64, dy as i64 + 1) { coverage = SEA_ICE_EDGE; }
            if BAYER_4X4[ry as usize % 4][rx as usize % 4] < coverage {
                let t = temperature.sample(nx, ny);
                let d = bayer_dither(t / params.sea_ice_temp_threshold, rx as usize, ry as usize, N_DITHER_LEVELS);
                sea_ice_color(d, params.sea_ice_temp_threshold)
            } else {
                let d = bayer_dither(hydro_nearest, rx as usize, ry as usize, N_DITHER_LEVELS).max(0.01);
                water_color(d)
            }
        } else if is_water {
            let d = bayer_dither(hydro_nearest, rx as usize, ry as usize, N_DITHER_LEVELS).max(0.01);
            water_color(d)
        } else if is_glacier[data_idx] {
            // Dither glacier edges against adjacent non-glaciated land, same pattern as water edges.
            let non_glacier_land = |ndx: i64, ndy: i64| -> bool {
                let nnx = ndx.rem_euclid(width as i64) as usize;
                let nny = ndy.clamp(0, height as i64 - 1) as usize;
                let nidx = nny * width + nnx;
                !is_glacier[nidx] && !is_ocean[nidx] && result.map.data[nidx] <= 0.0
            };
            const GLACIER_EDGE: f64 = 0.05;
            let mut coverage = 1.0f64;
            if off_x == 0 && non_glacier_land(dx as i64 - 1, dy as i64) { coverage = GLACIER_EDGE; }
            if off_x == 2 && non_glacier_land(dx as i64 + 1, dy as i64) { coverage = GLACIER_EDGE; }
            if off_y == 0 && non_glacier_land(dx as i64, dy as i64 - 1) { coverage = GLACIER_EDGE; }
            if off_y == 2 && non_glacier_land(dx as i64, dy as i64 + 1) { coverage = GLACIER_EDGE; }
            if BAYER_4X4[ry as usize % 4][rx as usize % 4] < coverage {
                let t = temperature.sample(nx, ny);
                let d = bayer_dither(t / params.glacier_temp_threshold, rx as usize, ry as usize, N_DITHER_LEVELS);
                glacier_color(d, params.glacier_temp_threshold)
            } else {
                let elev_t = elevation.sample(nx, ny);
                let land_t = ((elev_t - params.sea_level) / (1.0 - params.sea_level)).clamp(0.0, 1.0);
                let d = bayer_dither(land_t, rx as usize, ry as usize, N_DITHER_LEVELS);
                terrain_color(d)
            }
        } else {
            let t = elevation.sample(nx, ny);
            let land_t = ((t - params.sea_level) / (1.0 - params.sea_level)).clamp(0.0, 1.0);
            let d = bayer_dither(land_t, rx as usize, ry as usize, N_DITHER_LEVELS);
            terrain_color(d)
        };
        let nx_r = (rx as usize + 1) as f64 / render_width as f64;
        let ny_d = (ry as usize + 1) as f64 / render_height as f64;
        let e = elevation.sample(nx, ny);
        let e_r = elevation.sample(nx_r, ny);
        let e_d = elevation.sample(nx, ny_d);
        if is_contour(e, e_r, e_d, N_CONTOURS) {
            let factor = if is_water { CONTOUR_DARKEN_WATER } else { CONTOUR_DARKEN };
            color = [
                (color[0] as f64 * factor) as u8,
                (color[1] as f64 * factor) as u8,
                (color[2] as f64 * factor) as u8,
            ];
        }
        Rgb(color)
    });
    composite.save("composite.png").expect("failed to save composite.png");
    println!("Saved composite.png");

    // ── Region detection ─────────────────────────────────────────────────────
    println!(
        "Detecting regions (land_thr={}, ocean_thr={}, min_size={})...",
        params.land_threshold, params.ocean_threshold, params.region_min_size,
    );
    let (region_map, regions) = detect_regions(
        &elevation, &temperature, &precipitation, &aridity,
        &is_ocean, &is_glacier, &is_sea_ice,
        params.land_threshold, params.ocean_threshold, params.region_min_size,
        params.island_coast_dist, params.island_arch_dist,
        params.lon_weight,
    );

    let total_cells = (width * height) as f64;
    println!();
    println!("=== {} regions detected ===", regions.len());
    println!();
    println!("{:>4}  {:>7}  {:>6}  {:>5}  {:>5}  {:>6}  {:>5}  {}",
        "ID", "Cells", "%", "Elev", "Temp", "Precip", "Arid", "Character");
    println!("{}", "─".repeat(68));
    for r in &regions {
        println!("{:>4}  {:>7}  {:>5.1}%  {:>5.2}  {:>5.2}  {:>6.2}  {:>5.2}  {}",
            r.id,
            r.size,
            r.size as f64 / total_cells * 100.0,
            r.mean_elev,
            r.mean_temp,
            r.mean_precip,
            r.mean_aridity,
            r.character(),
        );
    }
    println!("{}", "─".repeat(68));
    println!();

    // Region composite: base composite with red outlines at region boundaries.
    println!("Rendering region map at {}x{}...", render_width, render_height);
    let region_composite = ImageBuffer::from_fn(render_width as u32, render_height as u32, |rx, ry| {
        let dx = rx as usize / RENDER_SCALE;
        let dy = ry as usize / RENDER_SCALE;
        let cur = region_map[dy * width + dx];
        let is_boundary = [(-1i64, 0i64), (1, 0), (0, -1), (0, 1)].iter().any(|&(ddx, ddy)| {
            let ndx = (dx as i64 + ddx).rem_euclid(width as i64) as usize;
            let ndy = (dy as i64 + ddy).clamp(0, height as i64 - 1) as usize;
            region_map[ndy * width + ndx] != cur
        });
        if is_boundary { Rgb([220u8, 30, 30]) } else { *composite.get_pixel(rx, ry) }
    });
    region_composite.save("regions.png").expect("failed to save regions.png");
    println!("Saved regions.png");

    // Political map: semi-transparent region colors blended onto the composite,
    // with darkened land-region borders and numeric ID labels at centroids.
    println!("Rendering political map at {}x{}...", render_width, render_height);

    // Compute each region's centroid in data-pixel space from the region map.
    // x uses a circular mean so dateline-spanning regions get a correct centre
    // rather than averaging to the middle of the map.
    let max_rid = regions.iter().map(|r| r.id as usize).max().unwrap_or(0) + 1;
    let mut cent_sin = vec![0.0f64; max_rid];
    let mut cent_cos = vec![0.0f64; max_rid];
    let mut cent_y   = vec![0u64;   max_rid];
    let mut cent_n   = vec![0u64;   max_rid];
    for (idx, &rid) in region_map.iter().enumerate() {
        if rid == u32::MAX { continue; }
        let angle = std::f64::consts::TAU * (idx % width) as f64 / width as f64;
        cent_sin[rid as usize] += angle.sin();
        cent_cos[rid as usize] += angle.cos();
        cent_y[rid as usize]   += (idx / width) as u64;
        cent_n[rid as usize]   += 1;
    }

    const ALPHA: f64 = 0.42;
    let mut political = ImageBuffer::from_fn(render_width as u32, render_height as u32, |rx, ry| {
        let dx = rx as usize / RENDER_SCALE;
        let dy = ry as usize / RENDER_SCALE;
        let data_idx = dy * width + dx;
        let base = composite.get_pixel(rx, ry);

        let cur = region_map[data_idx];
        if is_ocean[data_idx] || is_glacier[data_idx] || cur == u32::MAX {
            return *base;
        }
        let [or_, og, ob] = political_color(cur);

        let is_border = [(-1i64, 0i64), (1, 0), (0, -1), (0, 1)].iter().any(|&(ddx, ddy)| {
            let ndx = (dx as i64 + ddx).rem_euclid(width as i64) as usize;
            let ndy = (dy as i64 + ddy).clamp(0, height as i64 - 1) as usize;
            let nidx = ndy * width + ndx;
            !is_ocean[nidx] && !is_glacier[nidx] && region_map[nidx] != cur
        });

        if is_border {
            Rgb([base[0] / 3, base[1] / 3, base[2] / 3])
        } else {
            let b = |base_c: u8, over_c: u8| -> u8 {
                (base_c as f64 * (1.0 - ALPHA) + over_c as f64 * ALPHA).round() as u8
            };
            Rgb([b(base[0], or_), b(base[1], og), b(base[2], ob)])
        }
    });

    // Draw region ID labels centered on each region's centroid.
    const TEXT_SCALE: i32 = 2;
    for r in &regions {
        let rid = r.id as usize;
        if cent_n[rid] == 0 { continue; }
        // Circular mean for x resolves dateline-spanning regions correctly.
        let n = cent_n[rid] as f64;
        let mean_angle = (cent_sin[rid] / n).atan2(cent_cos[rid] / n);
        let cx_data = (mean_angle / std::f64::consts::TAU * width as f64)
            .rem_euclid(width as f64) as usize;
        let cx = cx_data * RENDER_SCALE + RENDER_SCALE / 2;
        let cy = (cent_y[rid] / cent_n[rid]) as usize * RENDER_SCALE + RENDER_SCALE / 2;
        let label = format!("{}", r.id);
        let label_w = label.len() as i32 * 8 * TEXT_SCALE;
        let lx = (cx as i32 - label_w / 2).clamp(0, render_width as i32 - label_w);
        let ly = cy as i32 - 4 * TEXT_SCALE;
        draw_text(&mut political, lx + 1, ly + 1, &label, [0, 0, 0], TEXT_SCALE);
        draw_text(&mut political, lx,     ly,     &label, [255, 255, 255], TEXT_SCALE);
    }

    political.save("political.png").expect("failed to save political.png");
    println!("Saved political.png");
}
