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

pub enum FoodTag {
    Herbivorous,
    Carnivorous,
    Photosynethic,
    Chemosynthetic,
    Lithotropic,
}

pub struct Range<T> {
    pub min: T,
    pub max: T,
}

pub struct Species {
    pub species_kind: SpeciesKind,
    pub life_basis: LifeBasis,
    pub solvent: Solvent,
}

fn main() {
    println!("Hello, world!");
}