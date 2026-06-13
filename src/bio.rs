use serde::{Serialize, Deserialize};
use uuid::Uuid;

use crate::common::Range;
use crate::universe::AtmosphereTag;

#[derive(Debug, Serialize, Deserialize)]
pub enum SpeciesKind {
    Named,
    Generic,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum SpeciesSentience {
    Sentient,
    PreSapient,
    Sapient,
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
pub enum ReproductiveRole {
    Contributor,
    Receiver,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SexKind {
    pub name: String,
    pub symbol: Option<String>,
    pub reproductive_role: Option<Vec<ReproductiveRole>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum ReproductionKind {
    Fission,
    Sporogenesis,
    Fragmentation,
    Agamogenesis,
    Sexual,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum ReproductiveMethod {   // Only applies if
    Ovuliparity,                // ReproductionKind is
    Zygoparity,                 // Sexual.
    Ovoviviparity,
    Viviparity,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ReproductionProfile {
    pub sex_kinds: Vec<SexKind>,
    pub repro_kind: Vec<ReproductionKind>,
    pub repro_method: Option<ReproductiveMethod>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Species {
    pub id: Uuid,
    pub name: Option<String>,
    pub kind: SpeciesKind,
    pub origin_world_id: Uuid,
    pub sentience: Option<SpeciesSentience>,
    pub basis: LifeBasis,
    pub solvent: Solvent,
    pub atmo_aff: Vec<AtmosphereAffinity>,
    pub food_tag: Vec<FoodTag>,
    pub repro_profile: ReproductionProfile,
    pub lifespan: Option<Range<u32>>,
    pub temp_range: Option<Range<f32>>,
    pub press_range: Option<Range<f32>>,
    pub grav_range: Option<Range<f32>>,
//  pub visibility: EntityVisibility,   // This will be defined for entities generally.
//  pinned: bool,
}
