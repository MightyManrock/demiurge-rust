use demiurge_rust::planet_gen::*;
use demiurge_rust::universe::{
    AtmosphereTag, CosmicCoordinates, EntityAge, Footprint, GeoTag,
    LiquidTag, Planet, Star, StarKind,
};
use demiurge_rust::bio::{
    ActivityPattern, Species, SpeciesKind, SpeciesSentience, AtmosphereAffinity,
    AtmosphereRelationship, FoodTag, LifeBasis, ReproductionKind,
    ReproductionProfile, ReproductiveMethod, ReproductiveRole,
    RespirationMedium, SexKind, Solvent,
};
use demiurge_rust::common::Range;
use uuid::Uuid;
use image::{ImageBuffer, Rgb};
use std::collections::HashMap;
use std::f64::consts::PI;

// ── Main ─────────────────────────────────────────────────────────────────────

fn main() {
    let width = 1024usize;
    let height = 512usize;

    // ── Planet selection ──────────────────────────────────────────────────────
    // Usage:
    //   cargo run --example heatmap_export                        # earth-like, seed 0
    //   cargo run --example heatmap_export -- earth <uuid|u32>   # earth-like, UUID or raw seed
    //   cargo run --example heatmap_export -- oros                # Oros
    let args: Vec<String> = std::env::args().collect();
    let mode = args.get(1).map(|s| s.as_str()).unwrap_or("earth");

    let (params, planet_name) = match mode {
        "oros" => (PlanetParams::oros(), "Oros".to_string()),
        _ => {
            let seed: u32 = match args.get(2).map(|s| s.as_str()) {
                Some(s) => {
                    if let Ok(uuid) = Uuid::parse_str(s) {
                        seed_from_uuid(*uuid.as_bytes())
                    } else if let Ok(n) = s.parse::<u32>() {
                        n
                    } else {
                        eprintln!("unrecognized seed '{}' — expected a UUID or u32, using 0", s);
                        0
                    }
                }
                None => 0,
            };
            (PlanetParams::earth_like(seed), "Earth-like".to_string())
        }
    };
    let seed = params.seed;

    println!("Planet: {} | temp_baseline={:.2} temp_gradient={:.2} precip_moisture={:.3} sea_level={:.2}",
        planet_name, params.temp_baseline, params.temp_gradient, params.precip_moisture, params.sea_level);

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
    let temperature      = HeatMap::generate_temperature(&elevation, &params, 0.0);
    let is_sea_ice       = generate_sea_ice(&temperature, &is_ocean, params.sea_ice_temp_threshold);
    // Four seasonal passes; annual-mean phasor bases drive the base hydrology.
    let rainfall_phasors = generate_seasonal_precip(&elevation, &is_ocean, &temperature, &is_sea_ice, &params);
    let precipitation    = HeatMap {
        width,
        height,
        data: rainfall_phasors.iter().map(|p| p.base as f64).collect(),
    };
    let is_glacier       = generate_glacier(&temperature, &is_ocean, params.glacier_temp_threshold);
    let aridity          = HeatMap::generate_aridity(&temperature, &precipitation, params.et_factor);
    let diurnal_swing    = generate_diurnal_swing(&temperature, &aridity, &params);

    let temp_img = ImageBuffer::from_fn(width as u32, height as u32, |x, y| {
        let nx = x as f64 / width as f64;
        let ny = y as f64 / height as f64;
        Rgb(temperature_color(temperature.sample(nx, ny)))
    });
    temp_img.save("temperature.png").expect("failed to save temperature.png");
    println!("Saved temperature.png");

    // Day/night temperature maps derived from the diurnal swing.
    for (label, sign) in [("day", 1.0_f64), ("night", -1.0_f64)] {
        let field_img = ImageBuffer::from_fn(width as u32, height as u32, |x, y| {
            let i = y as usize * width + x as usize;
            let t = (temperature.data[i] + sign * diurnal_swing.data[i] / 2.0).clamp(0.0, 1.0);
            Rgb(temperature_color(t))
        });
        let fname = format!("temperature_{}.png", label);
        field_img.save(&fname).unwrap_or_else(|_| panic!("failed to save {}", fname));
        println!("Saved {}", fname);
    }

    // Diurnal swing magnitude (day→night gap), scaled to Celsius for readability.
    {
        let max_gap_c = diurnal_swing.data.iter().cloned().fold(0.0_f64, f64::max) * 70.0;
        let swing_img = ImageBuffer::from_fn(width as u32, height as u32, |x, y| {
            let i = y as usize * width + x as usize;
            let gap_c = diurnal_swing.data[i] * 70.0;
            // Normalize against a 40°C reference for the color ramp.
            Rgb(temperature_color((gap_c / 40.0).clamp(0.0, 1.0)))
        });
        swing_img.save("diurnal_swing.png").expect("failed to save diurnal_swing.png");
        println!("Saved diurnal_swing.png (peak gap ≈ {:.0}°C)", max_gap_c);
    }

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

    println!("Generating seasonal hydrology...");
    let snowmelt_amp = classify_snowpack(&temperature, &is_ocean, &rainfall_phasors, &params);
    let snowpack_cells = snowmelt_amp.iter().filter(|&&a| a > 0.0).count();
    let seasonal_hydro = generate_seasonal_hydro(
        rainfall_phasors,
        snowmelt_amp,
        (3.0 * PI / 2.0) as f32,
        &result.flow_to,
        &result.topo_order,
        &result.accumulation,
    );
    println!(
        "  {} snowpack cells classified; seasonal phasors propagated",
        snowpack_cells
    );

    // Salt flats: endorheic basin floors that are arid enough and geologically
    // crystalline. salt_flat_probability raises the aridity threshold — higher
    // Crystalline fraction means more evaporite basins qualify.
    let salt_flat_aridity_threshold = 0.35 + params.salt_flat_probability * 0.25;
    let is_salt_flat: Vec<bool> = (0..width * height).map(|i| {
        result.is_endorheic[i]
            && !is_ocean[i]
            && !is_glacier[i]
            && aridity.data[i] < salt_flat_aridity_threshold
    }).collect();
    let n_salt_flat = is_salt_flat.iter().filter(|&&b| b).count();
    println!("  {} salt flat cells (aridity_thr={:.3})", n_salt_flat, salt_flat_aridity_threshold);

    let salt_flat_dist = compute_salt_flat_dist(&is_salt_flat, width, height);

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
    let composite = render_composite_map(
        width, height, render_width, render_height,
        &result.map, &elevation, &temperature, &aridity,
        &is_ocean, &is_glacier, &is_sea_ice, &is_salt_flat, &salt_flat_dist, &params,
    );
    composite.save("composite.png").expect("failed to save composite.png");
    println!("Saved composite.png");

    // ── Region detection ─────────────────────────────────────────────────────
    println!(
        "Detecting regions (land_thr={}, ocean_thr={}, min_size={})...",
        params.land_threshold, params.ocean_threshold, params.region_min_size,
    );
    let (region_map, regions) = detect_regions(
        &elevation, &temperature, &diurnal_swing, &precipitation, &aridity,
        &is_ocean, &is_glacier, &is_sea_ice, &is_salt_flat,
        params.land_threshold, params.ocean_threshold, params.region_min_size,
        params.island_coast_dist, params.island_arch_dist,
        params.lon_weight,
    );

    let keth = Species {
        id: Uuid::parse_str("2433c35f-6a41-4529-af95-92d9c1f1c4dc").unwrap(),
        name: Some("Keth".into()),
        kind: SpeciesKind::Named,
        origin_world_id: Uuid::parse_str("e3f92fd2-3501-40b4-957f-95d65dc4b51e").unwrap(),
        sentience: Some(SpeciesSentience::Sapient),
        basis: LifeBasis::Carbon,
        solvent: Solvent {
            liquid: LiquidTag::Water,
            access_range: Some(Range { min: 0.65, max: 0.95 }),  // 1-aridity on Oros ≈ 0.75-0.82
            humidity_range: Some(Range { min: 0.05, max: 0.70 }),
        },
        atmo_aff: vec![
            AtmosphereAffinity {
                tag: Some(AtmosphereTag::Oxygen),
                threshold: Some(Range { min: 0.11, max: 0.18 }),
                relationship: AtmosphereRelationship::Required,
                medium: RespirationMedium::Gas,
            },
            AtmosphereAffinity {
                tag: Some(AtmosphereTag::CarbonDioxide),
                threshold: Some(Range { min: 0.0, max: 0.03 }),
                relationship: AtmosphereRelationship::Tolerated,
                medium: RespirationMedium::Gas,
            },
            AtmosphereAffinity {
                tag: Some(AtmosphereTag::CarbonMonoxide),
                threshold: Some(Range { min: 0.002, max: 1.0 }),
                relationship: AtmosphereRelationship::Fatal,
                medium: RespirationMedium::Gas,
            },
            AtmosphereAffinity { tag: None, threshold: None, relationship: AtmosphereRelationship::Fatal, medium: RespirationMedium::Liquid },
            AtmosphereAffinity { tag: None, threshold: None, relationship: AtmosphereRelationship::Fatal, medium: RespirationMedium::Solid },
            AtmosphereAffinity { tag: None, threshold: None, relationship: AtmosphereRelationship::Fatal, medium: RespirationMedium::Vacuum },
        ],
        food_tag: vec![FoodTag::Carnivorous],
        repro_profile: ReproductionProfile {
            sex_kinds: vec![
                SexKind { name: "Male".into(),   symbol: None, reproductive_role: Some(vec![ReproductiveRole::Contributor]) },
                SexKind { name: "Female".into(), symbol: None, reproductive_role: Some(vec![ReproductiveRole::Receiver]) },
            ],
            repro_kind: vec![ReproductionKind::Sexual],
            repro_method: Some(ReproductiveMethod::Viviparity),
        },
        lifespan: Some(Range { min: 200, max: 280 }),
        temp_range: Some(Range { min: 22.0, max: 30.0 }),
        press_range: Some(Range { min: 55.0, max: 105.0 }),
        grav_range: Some(Range { min: 0.50, max: 1.10 }),
        activity: ActivityPattern::Nocturnal,
    };

    let species_list: &[(&str, &Species)] = &[("Keth", &keth)];

    let total_cells = (width * height) as f64;
    for (sp_name, species) in species_list {
        println!();
        println!("=== {} regions — suitability for {} ===", regions.len(), sp_name);
        println!();
        println!("{:>4}  {:>7}  {:>6}  {:>5}  {:>5}  {:>6}  {:>5}  {:>5}  {}",
            "ID", "Cells", "%", "Elev", "Temp", "Precip", "Arid", "Suit%", "Character");
        println!("{}", "─".repeat(75));
        for r in &regions {
            let suit = score_region_for_species(species, r, &params);
            println!("{:>4}  {:>7}  {:>5.1}%  {:>5.2}  {:>5.2}  {:>6.2}  {:>5.2}  {:>4.0}%  {}",
                r.id,
                r.size,
                r.size as f64 / total_cells * 100.0,
                r.mean_elev,
                r.mean_temp,
                r.mean_precip,
                r.mean_aridity,
                suit * 100.0,
                r.character(),
            );
        }
        println!("{}", "─".repeat(75));

        // Per-cell habitability heatmap at 3× with Bayer dithering.
        let suit_cells: Vec<f64> = (0..width * height).map(|i| {
            score_cell_for_species(
                species,
                elevation.data[i],
                temperature.data[i],
                precipitation.data[i],
                aridity.data[i],
                is_ocean[i],
                diurnal_swing.data[i],
                &params,
            )
        }).collect();
        let suit_img = ImageBuffer::from_fn(render_width as u32, render_height as u32, |rx, ry| {
            let dx = rx as usize / RENDER_SCALE;
            let dy = ry as usize / RENDER_SCALE;
            let raw = suit_cells[dy * width + dx];
            let d = bayer_dither(raw, rx as usize, ry as usize, N_DITHER_LEVELS);
            Rgb(habitability_color(d))
        });
        let fname = format!("habitability_{}.png", sp_name.to_lowercase());
        suit_img.save(&fname).unwrap_or_else(|_| panic!("failed to save {}", fname));
        println!("Saved {}", fname);
    }
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

    // ── Seasonal snapshot maps ────────────────────────────────────────────────
    //
    // Two composite + habitability renders at opposite ends of the year so
    // differences in river network, lake levels, and precipitation are visible.
    // Temperature and glaciers are static (no seasonal temperature model yet).
    for (season_name, season_phase) in [("summer", 0.0_f64), ("winter", PI)] {
        println!();
        println!("Rendering {} snapshot...", season_name);

        // Sample seasonal precipitation from phasors.
        let sea_precip = HeatMap {
            width,
            height,
            data: seasonal_hydro.rainfall.iter()
                .map(|p| sample_precip_phasor(p, season_phase) as f64)
                .collect(),
        };
        let sea_aridity = HeatMap::generate_aridity(&temperature, &sea_precip, params.et_factor);
        let sea_swing   = generate_diurnal_swing(&temperature, &sea_aridity, &params);

        // Re-run hydrology from seasonal precipitation so river/lake topology
        // reflects the actual flow volume at this time of year.
        let sea_result = HeatMap::generate_hydrology(
            &elevation, &is_ocean, &sea_precip, &is_glacier, &params,
        );

        // Recompute salt flats: drier season = more basins qualify.
        let salt_flat_aridity_threshold = 0.35 + params.salt_flat_probability * 0.25;
        let sea_salt_flat: Vec<bool> = (0..width * height).map(|i| {
            sea_result.is_endorheic[i]
                && !is_ocean[i]
                && !is_glacier[i]
                && sea_aridity.data[i] < salt_flat_aridity_threshold
        }).collect();
        let sea_salt_flat_dist = compute_salt_flat_dist(&sea_salt_flat, width, height);

        let sea_composite = render_composite_map(
            width, height, render_width, render_height,
            &sea_result.map, &elevation, &temperature, &sea_aridity,
            &is_ocean, &is_glacier, &is_sea_ice, &sea_salt_flat, &sea_salt_flat_dist, &params,
        );
        let composite_name = format!("composite_{}.png", season_name);
        sea_composite.save(&composite_name).unwrap_or_else(|_| panic!("failed to save {}", composite_name));
        println!("Saved {}", composite_name);

        // Habitability maps for each species at this season.
        const N_DITHER_LEVELS: usize = 16;
        const RENDER_SCALE: usize = 3;
        for (sp_name, species) in species_list {
            let suit_cells: Vec<f64> = (0..width * height).map(|i| {
                score_cell_for_species(
                    species,
                    elevation.data[i],
                    temperature.data[i],
                    sea_precip.data[i],
                    sea_aridity.data[i],
                    is_ocean[i],
                    sea_swing.data[i],
                    &params,
                )
            }).collect();
            let suit_img = image::ImageBuffer::from_fn(render_width as u32, render_height as u32, |rx, ry| {
                let dx = rx as usize / RENDER_SCALE;
                let dy = ry as usize / RENDER_SCALE;
                let raw = suit_cells[dy * width + dx];
                let d = bayer_dither(raw, rx as usize, ry as usize, N_DITHER_LEVELS);
                image::Rgb(habitability_color(d))
            });
            let fname = format!("habitability_{}_{}.png", sp_name.to_lowercase(), season_name);
            suit_img.save(&fname).unwrap_or_else(|_| panic!("failed to save {}", fname));
            println!("Saved {}", fname);
        }
    }
}
