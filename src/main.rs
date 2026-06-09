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
    Oyxgen,
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

fn main() {
    println!("Hello, world!");
}