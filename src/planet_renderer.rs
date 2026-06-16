use godot::prelude::*;
use crate::planet_gen::{
    HeatMap, PlanetParams,
    generate_seasonal_precip,
    generate_sea_ice, generate_glacier, flood_fill_ocean,
    compute_salt_flat_dist, render_composite_map, sample_precip_phasor,
    render_terrain_layer, render_hydrology_layer, render_ice_layer,
};
use std::f64::consts::PI;

const WIDTH: usize = 512;
const HEIGHT: usize = 256;
const SEASON_PHASES: [f64; 4] = [0.0, 0.25, 0.5, 0.75];

#[derive(GodotClass)]
#[class(base=Node)]
pub struct PlanetMapRenderer {
    // Stable data
    params:          Option<PlanetParams>,
    elevation:       Option<HeatMap>,
    aridity:         Option<HeatMap>,
    is_ocean:        Option<Vec<bool>>,
    is_salt_flat:    Option<Vec<bool>>,
    salt_flat_dist:  Option<Vec<u32>>,
    annual_hydro:    Option<HeatMap>,

    // Seasonal snapshots (4 per layer)
    seasonal_temps:      Vec<HeatMap>,
    seasonal_hydro_maps: Vec<HeatMap>,
    seasonal_glacier:    Vec<Vec<bool>>,
    seasonal_sea_ice:    Vec<Vec<bool>>,

    base: Base<Node>,
}

#[godot_api]
impl INode for PlanetMapRenderer {
    fn init(base: Base<Node>) -> Self {
        Self {
            params: None, elevation: None, aridity: None,
            is_ocean: None, is_salt_flat: None, salt_flat_dist: None,
            annual_hydro: None,
            seasonal_temps: Vec::new(),
            seasonal_hydro_maps: Vec::new(),
            seasonal_glacier: Vec::new(),
            seasonal_sea_ice: Vec::new(),
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
        // Annual mean (phase 0) for stable layers
        let annual_temp      = HeatMap::generate_temperature(&elevation, &params, 0.0);
        let annual_sea_ice   = generate_sea_ice(&annual_temp, &is_ocean, params.sea_ice_temp_threshold);
        let rainfall_phasors = generate_seasonal_precip(
            &elevation, &is_ocean, &annual_temp, &annual_sea_ice, &params,
        );
        let annual_precip    = HeatMap {
            width: WIDTH, height: HEIGHT,
            data: rainfall_phasors.iter().map(|p| p.base as f64).collect(),
        };
        let annual_glacier   = generate_glacier(&annual_temp, &is_ocean, params.glacier_temp_threshold);
        let aridity          = HeatMap::generate_aridity(&annual_temp, &annual_precip, params.et_factor);
        let annual_result    = HeatMap::generate_hydrology(
            &elevation, &is_ocean, &annual_precip, &annual_glacier, &params,
        );

        let salt_threshold = 0.35 + params.salt_flat_probability * 0.25;
        let is_salt_flat: Vec<bool> = (0..WIDTH * HEIGHT).map(|i| {
            annual_result.is_endorheic[i] && !is_ocean[i] && !annual_glacier[i]
                && aridity.data[i] < salt_threshold
        }).collect();
        let salt_flat_dist = compute_salt_flat_dist(&is_salt_flat, WIDTH, HEIGHT);

        // Build 4 seasonal snapshots
        let mut seasonal_temps      = Vec::with_capacity(4);
        let mut seasonal_hydro_maps = Vec::with_capacity(4);
        let mut seasonal_glacier    = Vec::with_capacity(4);
        let mut seasonal_sea_ice    = Vec::with_capacity(4);

        for &phase in &SEASON_PHASES {
            let s_temp    = HeatMap::generate_temperature(&elevation, &params, phase);
            let s_glacier = generate_glacier(&s_temp, &is_ocean, params.glacier_temp_threshold);
            let s_sea_ice = generate_sea_ice(&s_temp, &is_ocean, params.sea_ice_temp_threshold);

            let season_angle = phase * 2.0 * PI;
            let s_precip_data: Vec<f64> = rainfall_phasors.iter()
                .map(|p| sample_precip_phasor(p, season_angle) as f64)
                .collect();
            let s_precip = HeatMap { width: WIDTH, height: HEIGHT, data: s_precip_data };
            let s_hydro  = HeatMap::generate_hydrology(
                &elevation, &is_ocean, &s_precip, &s_glacier, &params,
            );

            seasonal_temps.push(s_temp);
            seasonal_hydro_maps.push(s_hydro.map);
            seasonal_glacier.push(s_glacier);
            seasonal_sea_ice.push(s_sea_ice);
        }

        self.annual_hydro        = Some(annual_result.map);
        self.params              = Some(params);
        self.elevation           = Some(elevation);
        self.aridity             = Some(aridity);
        self.is_ocean            = Some(is_ocean);
        self.is_salt_flat        = Some(is_salt_flat);
        self.salt_flat_dist      = Some(salt_flat_dist);
        self.seasonal_temps      = seasonal_temps;
        self.seasonal_hydro_maps = seasonal_hydro_maps;
        self.seasonal_glacier    = seasonal_glacier;
        self.seasonal_sea_ice    = seasonal_sea_ice;
    }

    #[func]
    pub fn terrain_texture(&self) -> PackedByteArray {
        let p  = self.params.as_ref().expect("call initialize() first");
        let img = render_terrain_layer(
            WIDTH, HEIGHT, WIDTH * 3, HEIGHT * 3,
            self.elevation.as_ref().unwrap(),
            self.aridity.as_ref().unwrap(),
            self.is_ocean.as_ref().unwrap(),
            self.is_salt_flat.as_ref().unwrap(),
            self.salt_flat_dist.as_ref().unwrap(),
            p,
        );
        PackedByteArray::from(img.into_raw().as_slice())
    }

    #[func]
    pub fn hydrology_texture(&self, snapshot: i32) -> PackedByteArray {
        let idx = snapshot.clamp(0, 3) as usize;
        let img = render_hydrology_layer(
            WIDTH, HEIGHT, WIDTH * 3, HEIGHT * 3,
            &self.seasonal_hydro_maps[idx],
        );
        PackedByteArray::from(img.into_raw().as_slice())
    }

    #[func]
    pub fn ice_texture(&self, snapshot: i32) -> PackedByteArray {
        let idx = snapshot.clamp(0, 3) as usize;
        let p   = self.params.as_ref().expect("call initialize() first");
        let img = render_ice_layer(
            WIDTH, HEIGHT, WIDTH * 3, HEIGHT * 3,
            &self.seasonal_temps[idx],
            &self.seasonal_glacier[idx],
            &self.seasonal_sea_ice[idx],
            self.is_ocean.as_ref().unwrap(),
            self.annual_hydro.as_ref().unwrap(),
            p,
        );
        PackedByteArray::from(img.into_raw().as_slice())
    }

    #[func]
    pub fn annual_texture(&self) -> PackedByteArray {
        let p   = self.params.as_ref().expect("call initialize() first");
        let img = render_composite_map(
            WIDTH, HEIGHT, WIDTH * 3, HEIGHT * 3,
            self.annual_hydro.as_ref().unwrap(),
            self.elevation.as_ref().unwrap(),
            &self.seasonal_temps[0],
            self.aridity.as_ref().unwrap(),
            self.is_ocean.as_ref().unwrap(),
            &self.seasonal_glacier[0],
            &self.seasonal_sea_ice[0],
            self.is_salt_flat.as_ref().unwrap(),
            self.salt_flat_dist.as_ref().unwrap(),
            p,
        );
        PackedByteArray::from(img.into_raw().as_slice())
    }

    #[func]
    pub fn update_interval_ticks(&self) -> i64 {
        // 1 tick = 1 Oros day; orbital period 536 days; update ~monthly
        536 / 12
    }
}
