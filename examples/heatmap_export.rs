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
            sea_level: 0.45,
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

        let mut data = Vec::with_capacity(width * height);
        for y in 0..height {
            for x in 0..width {
                let nx = x as f64 / width as f64 * 3.5;
                let ny = y as f64 / height as f64 * 2.0;
                data.push(fbm.get([nx, ny]));
            }
        }

        // Normalize to [0, 1] using actual min/max.
        let min = data.iter().cloned().fold(f64::INFINITY, f64::min);
        let max = data.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        let range = max - min;
        for v in &mut data {
            *v = (*v - min) / range;
        }

        HeatMap { width, height, data }
    }

    fn generate_hydrology(elevation: &HeatMap, params: &HydrologyParams) -> HydrologyResult {
        let width = elevation.width;
        let height = elevation.height;
        let n = width * height;

        // ── Ocean flood-fill ─────────────────────────────────────────────────
        //
        // Rather than treating every cell below sea level as ocean, we BFS from
        // the global minimum and spread only to connected cells below sea level.
        // Any below-sea-level area not reachable from the lowest point is an
        // inland basin (dead sea, salt flat) — not ocean.
        let is_ocean = flood_fill_ocean(&elevation.data, width, height, params.sea_level);

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

        // ── Phase 3: Flow accumulation ──────────────────────────────────────
        //
        // Process land cells highest-first. Each cell adds its running
        // accumulation to its downstream neighbor. Cells with high accumulation
        // are rivers; cells with low accumulation are dry hillsides.

        let mut land_order: Vec<usize> = (0..n)
            .filter(|&i| !is_ocean[i])
            .collect();
        land_order.sort_unstable_by(|&a, &b| filled[b].total_cmp(&filled[a]));

        let mut accumulation = vec![1.0f64; n];
        for &idx in &land_order {
            if let Some(ds) = flow_to[idx] {
                accumulation[ds] += accumulation[idx];
            }
        }

        // ── Phase 4: Classify cells and identify aquifer zones ──────────────
        //
        // A cell whose filled elevation exceeds its natural elevation by more
        // than max_lake_fill is in a basin too deep to form a surface lake —
        // it becomes endorheic. Among cells that collected significant flow
        // before going endorheic, some become aquifer recharge zones (water
        // sinks underground) and the rest are terminal dry sinks (water runs
        // out). Both outcomes are placeholders: the real split should come from
        // climate (aridity → more dry sinks) and geology data once those maps
        // exist.
        //
        // Note: the endorheic check is per-cell rather than per-basin. This
        // means a large deep lake appears as shallow water at its rim and dry
        // at its floor, which is a simplification. Per-basin classification
        // requires connected-component analysis and can be refined later.

        let mut aquifer_zones = Vec::new();
        let endorheic: Vec<bool> = (0..n)
            .map(|i| {
                !is_ocean[i] && filled[i] - elevation.data[i] > params.max_lake_fill
            })
            .collect();

        for idx in 0..n {
            if endorheic[idx] && accumulation[idx] >= params.river_threshold {
                if cell_hash(idx % width, idx / width) < params.aquifer_probability {
                    aquifer_zones.push((idx % width, idx / width));
                }
            }
        }

        // ── Phase 5: Encode into hydrology HeatMap ──────────────────────────
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

    println!("Generating {}x{} elevation map (seed {})...", width, height, seed);
    let elevation = HeatMap::generate_elevation(width, height, seed);

    let elev_img = ImageBuffer::from_fn(width as u32, height as u32, |x, y| {
        let nx = x as f64 / width as f64;
        let ny = y as f64 / height as f64;
        Rgb(elevation_color(elevation.sample(nx, ny)))
    });
    elev_img.save("elevation.png").expect("failed to save elevation.png");
    println!("Saved elevation.png");

    println!("Generating hydrology...");
    let params = HydrologyParams::default();
    let result = HeatMap::generate_hydrology(&elevation, &params);
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

    // Composite: elevation as base, water overlay on top.
    let composite = ImageBuffer::from_fn(width as u32, height as u32, |x, y| {
        let nx = x as f64 / width as f64;
        let ny = y as f64 / height as f64;
        let hydro = result.map.sample(nx, ny);
        let color = if hydro > 0.0 {
            water_color(hydro)
        } else {
            elevation_color(elevation.sample(nx, ny))
        };
        Rgb(color)
    });
    composite.save("composite.png").expect("failed to save composite.png");
    println!("Saved composite.png");
}

