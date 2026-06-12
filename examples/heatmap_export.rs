use image::{ImageBuffer, Rgb};
use noise::{Fbm, NoiseFn, Perlin};
use std::cmp::Reverse;
use std::collections::{BinaryHeap, VecDeque};

// ── Data structures ──────────────────────────────────────────────────────────

/// A 2D float field over a normalized [0,1) x [0,1) coordinate plane.
/// x wraps (east-west); y clamps (poles do not connect).
struct HeatMap {
    width: usize,
    height: usize,
    data: Vec<f64>,
}

struct HydrologyParams {
    /// Elevation threshold below which the planet surface is ocean [0, 1].
    sea_level: f64,
    /// Maximum fill depth for a lake. Depressions requiring deeper fill are
    /// treated as endorheic basins (water either runs out or sinks underground).
    max_lake_fill: f64,
    /// Fraction of endorheic basin cells that become aquifer recharge zones
    /// rather than terminal dry sinks. Placeholder until climate/geology data
    /// can drive this properly.
    aquifer_probability: f64,
    /// Minimum upstream cell count for a cell to render as a river.
    river_threshold: f64,
}

impl Default for HydrologyParams {
    fn default() -> Self {
        Self {
            sea_level: 0.5,
            max_lake_fill: 0.04,
            aquifer_probability: 0.35,
            river_threshold: 400.0,
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
    fn generate_elevation(width: usize, height: usize, seed: u32) -> Self {
        let fbm = Fbm::<Perlin>::new(seed);
        // Two decorrelated FBM fields warp the sample coordinates before the
        // main noise is read. This breaks up annular saddle features that FBM
        // occasionally produces, which otherwise manifest as ring-shaped trenches
        // that fill with circuit rivers. Spatial offsets (5.2, 1.3) decorrelate
        // the two warp axes from each other and from the main field.
        let warp_a = Fbm::<Perlin>::new(seed.wrapping_add(1));
        let warp_b = Fbm::<Perlin>::new(seed.wrapping_add(2));
        let warp_c = Fbm::<Perlin>::new(seed.wrapping_add(3));
        const WARP_STRENGTH: f64 = 0.2;
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
                let dx = warp_a.get([sx, sy, sz]) * WARP_STRENGTH;
                let dy = warp_b.get([sx + 5.2, sy + 1.3, sz + 3.7]) * WARP_STRENGTH;
                let dz = warp_c.get([sx + 2.8, sy + 4.6, sz + 1.9]) * WARP_STRENGTH;
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

    /// Latitude cosine + elevation lapse rate. No ocean moderation yet.
    fn generate_temperature(elevation: &HeatMap) -> HeatMap {
        let width = elevation.width;
        let height = elevation.height;
        const LAPSE_FACTOR: f64 = 0.3;

        let data = (0..width * height)
            .map(|idx| {
                let y = idx / width;
                let abs_lat =
                    (y as f64 - height as f64 / 2.0).abs() / (height as f64 / 2.0);
                let lat_base = (abs_lat * std::f64::consts::FRAC_PI_2).cos();
                (lat_base - elevation.data[idx] * LAPSE_FACTOR).clamp(0.0, 1.0)
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
    fn generate_precipitation(elevation: &HeatMap, is_ocean: &[bool]) -> HeatMap {
        let width = elevation.width;
        let height = elevation.height;
        let n = width * height;

        // Per land-cell moisture decay and rain-shadow factor.
        const LAND_DECAY: f64 = 0.985;
        // Only count elevation gain above this floor as a rain shadow — small
        // FBM noise between adjacent cells should not strip moisture.
        const SLOPE_THRESHOLD: f64 = 0.015;
        const SLOPE_LOSS: f64 = 0.5;
        // Minimum precipitation from local convection; keeps interiors non-zero.
        const BASE_ARID: f64 = 0.05;

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
                    carry = 1.0;
                } else {
                    let upwind_x = (x + width - 1) % width;
                    let raw_gain =
                        elevation.data[idx] - elevation.data[y * width + upwind_x];
                    let elev_gain = (raw_gain - SLOPE_THRESHOLD).max(0.0);
                    carry = (carry * LAND_DECAY - elev_gain * SLOPE_LOSS).max(0.0);
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
                    carry = 1.0;
                } else {
                    let upwind_x = (x + 1) % width;
                    let raw_gain =
                        elevation.data[idx] - elevation.data[y * width + upwind_x];
                    let elev_gain = (raw_gain - SLOPE_THRESHOLD).max(0.0);
                    carry = (carry * LAND_DECAY - elev_gain * SLOPE_LOSS).max(0.0);
                }
                if pass_i >= width {
                    moisture_east[idx] = carry;
                }
            }
        }

        let data = (0..n)
            .map(|idx| {
                let y = idx / width;
                let abs_lat =
                    (y as f64 - height as f64 / 2.0).abs() / (height as f64 / 2.0);
                let w = westerly_weight(abs_lat);
                let moisture = moisture_west[idx] * w + moisture_east[idx] * (1.0 - w);
                let band = lat_band_factor(abs_lat);
                (band * (BASE_ARID + moisture * (1.0 - BASE_ARID))).clamp(0.0, 1.0)
            })
            .collect();

        HeatMap { width, height, data }
    }

    fn generate_hydrology(
        elevation: &HeatMap,
        is_ocean: &[bool],
        precipitation: &HeatMap,
        params: &HydrologyParams,
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
        let mut accumulation: Vec<f64> = (0..n)
            .map(|i| {
                if is_ocean[i] { 0.0 } else { precipitation.data[i] / mean_land_precip }
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

// ── Main ─────────────────────────────────────────────────────────────────────

fn main() {
    let width = 1024usize;
    let height = 512usize;
    let seed = 42u32;
    const RENDER_SCALE: usize = 3;
    const N_DITHER_LEVELS: usize = 16;
    let render_width = width * RENDER_SCALE;
    let render_height = height * RENDER_SCALE;

    let params = HydrologyParams::default();

    println!("Generating {}x{} elevation map (seed {})...", width, height, seed);
    let mut elevation = HeatMap::generate_elevation(width, height, seed);
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
    let temperature = HeatMap::generate_temperature(&elevation);
    let precipitation = HeatMap::generate_precipitation(&elevation, &is_ocean);

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

    println!("Generating hydrology...");
    let result = HeatMap::generate_hydrology(&elevation, &is_ocean, &precipitation, &params);
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
        // Nearest-neighbor for classification avoids bilinear-interpolated halos
        // around water cells that would otherwise render as near-white water color.
        let is_water = result.map.sample_nearest(nx, ny) > 0.0;
        let mut color = if is_water {
            let hydro = result.map.sample(nx, ny);
            let d = bayer_dither(hydro, rx as usize, ry as usize, N_DITHER_LEVELS).max(0.01);
            water_color(d)
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
}
