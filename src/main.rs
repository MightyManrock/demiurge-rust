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
    Toxic
}


pub enum RespirationMedium {
    Gas,
    Liquid,
    Solid,
    Vacuum,
}

pub struct AtmosphereAffinity {
    pub tag: AtmosphereTag,
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

pub struct Range<T> {
    pub min: T,
    pub max: T,
}

pub struct Species {
    pub name: Option<String>,
    pub kind: SpeciesKind,
    pub life_basis: LifeBasis,
    pub solvent: Solvent,
    pub atmo_aff: Vec<AtmosphereAffinity>,
}

fn main() {
    println!("Hello, world!");
}