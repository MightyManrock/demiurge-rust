use crate::common::Range;
use serde::{Serialize, Deserialize};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize)]
pub enum SpeciesKind {
    Named,
    Generic,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum LifeBasis {
    Carbon,
    Silicon,
    Arsenic,
    Borane,
    Sulfur,
    Phosphorus,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum Solvent {
    Water,
    Ammonia,
    Methane,
    HydrogenFluoride,
    HydrogenSulfide,
    SiliconDioxide,
    SulfuricAcid,
    Formamide,
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
pub enum AtmosphereRelationship {
    Required,
    Beneficial,
    Tolerated,
    Toxic,
    Fatal,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum RespirationMedium {
    Gas,
    Liquid,
    Solid,
    Vacuum,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AtmosphereAffinity {
    pub tag: Option<AtmosphereTag>,
    pub relationship: AtmosphereRelationship,
    pub medium: RespirationMedium,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum FoodTag {
    Herbivorous,
    Carnivorous,
    Photosynthetic,
    Chemosynthetic,
    Lithotrophic,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Species {
    pub id: Uuid,
    pub name: Option<String>,
    pub kind: SpeciesKind,
    pub basis: LifeBasis,
    pub solvent: Solvent,
    pub atmo_aff: Vec<AtmosphereAffinity>,
    pub food_tag: Vec<FoodTag>,
    pub temp_range: Option<Range<f32>>,
    pub press_range: Option<Range<f32>>,
    pub grav_range: Option<Range<f32>>,
}