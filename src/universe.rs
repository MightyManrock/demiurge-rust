use serde::{Serialize, Deserialize};
use uuid::Uuid;
use std::collections::HashMap;

use crate::dom_cult_reg::DomainTag;

#[derive(Debug, Serialize, Deserialize, Hash, Eq, PartialEq)]
pub enum FootprintKind {
    DirectCreation,
    OvertMiracles,
    SubtleInfluence,
    ProxiusActivity,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Footprint {
    pub kind: HashMap<FootprintKind, f32>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct EntityAge {
    pub formation_billions: Option<u32>,
    pub formation_millions: Option<u32>,
    pub formation_thousands: Option<u32>,
    pub formation_years: u32,
    pub formation_month: u32,
    pub formation_day: u32,
    pub age_billions: Option<u32>,
    pub age_millions: Option<u32>,
    pub age_thousands: Option<u32>,
    pub age_years: Option<u32>,
    pub age_months: Option<u32>,
    pub age_days: u32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Universe {
    pub id: Uuid,
    pub name: String,
    pub age: EntityAge,
    pub domain_exp: HashMap<DomainTag, f32>,
    pub footprint: Footprint,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CosmicCoordinates {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GalacticCore {
    pub domain_exp: HashMap<DomainTag, f32>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Galaxy {
    pub id: Uuid,
    pub name: String,
    pub age: EntityAge,
    pub core_id: Uuid,
    pub parent_id: Uuid,
    pub child_ids: Option<Vec<Uuid>>,
    pub coord: CosmicCoordinates,
    pub domain_exp: HashMap<DomainTag, f32>,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum StarKind {
    BlueGiant,
    WhiteStar,
    YellowDwarf,
    OrangeDwarf,
    RedDwarf,
    RedGiant,
    WhiteDwarf,
    NeutronStar,
    BlackHole,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Star {
    pub id: Uuid,
    pub name: String,
    pub age: EntityAge,
    pub kind: StarKind,
    pub luminosity: f32,
    pub parent_id: Option<Uuid>,        // System ID
    pub companion_ids: Option<Vec<Uuid>>, // Other Star IDs
    pub domain_exp: HashMap<DomainTag, f32>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct System {
    pub id: Uuid,
    pub name: String,
    pub age: EntityAge,
    pub star_ids: Vec<Uuid>,
    pub parent_id: Option<Uuid>,
    pub child_ids: Option<Vec<Uuid>>,
    pub coord: CosmicCoordinates,
    pub domain_exp: HashMap<DomainTag, f32>,
}

#[derive(Debug, Serialize, Deserialize, Hash, Eq, PartialEq)]
pub enum AtmosphereTag {
    Oxygen,
    Nitrogen,
    CarbonMonoxide,
    CarbonDioxide,
    Methane,
    SulfurDioxide,
    WaterVapor,
    Ammonia,
}

#[derive(Debug, Serialize, Deserialize, Hash, Eq, PartialEq)]
pub enum LiquidTag {
    Water,
    Ammonia,
    Methane,
    HydrogenFluoride,
    HydrogenSulfide,
    SiliconDioxide,
    SulfuricAcid,
    Formamide,
    Ethane,
}

#[derive(Debug, Serialize, Deserialize, Hash, Eq, PartialEq)]
pub enum GeoTag {
    Silicate,
    Carbonate,
    Basaltic,
    Ferrous,
    Icy,
    Crystalline,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Planet {
    pub id: Uuid,
    pub name: String,
    pub age: EntityAge,
    pub parent_id: Option<Uuid>,
    pub child_ids: Option<Vec<Uuid>>,
    pub coord: CosmicCoordinates,
    pub radius: f32,
    pub gravity: f32,
    pub base_press: f32,
    pub axial_tilt: f32,
    pub atmo: HashMap<AtmosphereTag, f32>,
    pub geo: HashMap<GeoTag, f32>,
    pub volcanism: f32,
    pub hydro: HashMap<LiquidTag, f32>,
    pub liquid_coverage: f32,
    pub civ_ids: Option<Vec<Uuid>>,
    pub species_ids: Option<Vec<Uuid>>,
    pub domain_exp: HashMap<DomainTag, f32>,
    pub footprint: Footprint,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum PopLocationKind {
    Surface,
    Subterranean,
    Aquatic,
    Orbital,
    DeepSpace,
    Airborne,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PopLocation {
    pub id: Uuid,
    pub name: String,
    pub age: EntityAge,
    pub kind: PopLocationKind,
    pub parent_id: Uuid,
    pub atmo: Option<Vec<AtmosphereTag>>,
    pub geo: Option<Vec<GeoTag>>,
    pub faction_ids: Option<Vec<Uuid>>,
    pub band_ids: Option<Vec<Uuid>>,
    pub pop_ids: Option<Vec<Uuid>>,
    pub mortal_ids: Option<Vec<Uuid>>,
    pub domain_exp: HashMap<DomainTag, f32>,
    pub footprint: Footprint,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TravelLocation {
    pub id: Uuid,
    pub name: String,
    pub age: EntityAge,
    pub endpoint_ids: (Uuid, Uuid),
    pub band_ids: Option<Vec<Uuid>>,
    pub pop_ids: Option<Vec<Uuid>>,
    pub mortal_ids: Option<Vec<Uuid>>,
}
