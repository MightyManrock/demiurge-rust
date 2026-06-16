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
