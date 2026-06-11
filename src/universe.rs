use serde::{Serialize, Deserialize};
use uuid::Uuid;
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize, Hash, Eq, PartialEq)]
pub enum Domain {
    Truth,
    Order,
    Silence,
    Change,
    Conflict,
    Fire,
    Water,
    Void,
    Growth,
    Decay,
    Memory,
    Sacrifice,
    Light,
    Mastery,
    Secrecy,
    Community,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Footprint {
    pub direct: f64,
    pub overt: f64,
    pub subtle: f64,
    pub proxius: f64,
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
    pub domain_exp: HashMap<Domain, f64>,
    pub footprint: Footprint,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CosmicCoordinates {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GalacticCore {
    pub domain_exp: HashMap<Domain, f64>,
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
    pub domain_exp: HashMap<Domain, f64>,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum StarType {

}

#[derive(Debug, Serialize, Deserialize)]
pub struct Star {
    pub id: Uuid,
    pub name: String,
    pub age: EntityAge,
    pub _type: StarType,
    pub parent_id: Option<Uuid>,        // System ID
    pub companion_ids: Option<Vec<Uuid>>, // Other Star IDs
    pub domain_exp: HashMap<Domain, f64>,
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
    pub domain_exp: HashMap<Domain, f64>,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum AtmosphereTag {
    Oxygen,
    Nitrogen,
    CarbonMonoxide,
    CarbonDioxide,
    Methane,
    Fumarate,
    Sulfate,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum GeoTag {

}

#[derive(Debug, Serialize, Deserialize)]
pub struct Planet {
    pub id: Uuid,
    pub name: String,
    pub age: EntityAge,
    pub parent_id: Option<Uuid>,
    pub child_ids: Option<Vec<Uuid>>,
    pub coord: CosmicCoordinates,
    pub atmo: Option<Vec<AtmosphereTag>>,
    pub geo: Option<Vec<GeoTag>>,
    pub civ_ids: Option<Vec<Uuid>>,
    pub species_ids: Option<Vec<Uuid>>,
    pub domain_exp: HashMap<Domain, f64>,
    pub footprint: Footprint,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PopLocation {
    pub id: Uuid,
    pub name: String,
    pub age: EntityAge,
    pub parent_id: Uuid,
    pub atmo: Option<Vec<AtmosphereTag>>,
    pub geo: Option<Vec<GeoTag>>,
    pub faction_ids: Option<Vec<Uuid>>,
    pub band_ids: Option<Vec<Uuid>>,
    pub pop_ids: Option<Vec<Uuid>>,
    pub mortal_ids: Option<Vec<Uuid>>,
    pub domain_exp: HashMap<Domain, f64>,
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