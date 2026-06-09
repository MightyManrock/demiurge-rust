use crate::common::Range;

pub enum SpeciesKind {
    Named,
    Generic,
}

pub enum LifeBasis {
    Carbon,
    Silicon,
    Arsenic,
    Borane,
    Sulfur,
    Phosphorus,
}

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

pub enum AtmosphereTag {
    Oxygen,
    Nitrogen,
    CarbonMonoxide,
    CarbonDioxide,
    Methane,
    Fumarate,
    Sulfate,
}

pub enum AtmosphereRelationship {
    Required,
    Beneficial,
    Tolerated,
    Toxic,
    Fatal,
}

pub enum RespirationMedium {
    Gas,
    Liquid,
    Solid,
    Vacuum,
}

pub struct AtmosphereAffinity {
    pub tag: Option<AtmosphereTag>,
    pub relationship: AtmosphereRelationship,
    pub medium: RespirationMedium,
}

pub enum FoodTag {
    Herbivorous,
    Carnivorous,
    Photosynthetic,
    Chemosynthetic,
    Lithotrophic,
}

pub struct Species {
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