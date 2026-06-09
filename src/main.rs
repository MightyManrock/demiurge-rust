#![allow(dead_code)]

mod common;
mod bio;

fn main() {

    let my_species = bio::Species {
        name: Some(String::from("Human")),
        kind: bio::SpeciesKind::Named,
        basis: bio::LifeBasis::Carbon,
        solvent: bio::Solvent::Water,
        atmo_aff: vec![
            bio::AtmosphereAffinity {
                tag: Some(bio::AtmosphereTag::Oxygen),
                relationship: bio::AtmosphereRelationship::Required,
                medium: bio::RespirationMedium::Gas,
            },
            bio::AtmosphereAffinity {
                tag: Some(bio::AtmosphereTag::CarbonMonoxide),
                relationship: bio::AtmosphereRelationship::Toxic,
                medium: bio::RespirationMedium::Gas,
            },
            bio::AtmosphereAffinity {
                tag: Some(bio::AtmosphereTag::Methane),
                relationship: bio::AtmosphereRelationship::Fatal,
                medium: bio::RespirationMedium::Gas,
            },
            bio::AtmosphereAffinity {
                tag: Some(bio::AtmosphereTag::CarbonDioxide),
                relationship: bio::AtmosphereRelationship::Tolerated,
                medium: bio::RespirationMedium::Gas,
            },
            bio::AtmosphereAffinity {
                tag: None,
                relationship: bio::AtmosphereRelationship::Fatal,
                medium: bio::RespirationMedium::Liquid,
            },
            bio::AtmosphereAffinity {
                tag: None,
                relationship: bio::AtmosphereRelationship::Fatal,
                medium: bio::RespirationMedium::Solid,
            },
            bio::AtmosphereAffinity {
                tag: None,
                relationship: bio::AtmosphereRelationship::Fatal,
                medium: bio::RespirationMedium::Vacuum,
            },
        ],
        food_tag: vec![
            bio::FoodTag::Herbivorous,
            bio::FoodTag::Carnivorous,
        ],
        temp_range: Some(common::Range { min: 20.0, max: 25.0 }),
        press_range: Some(common::Range { min: 80.0, max: 120.0 }),
        grav_range: Some(common::Range { min: 0.35, max: 2.25 })
    };

    println!("{:#?}", my_species);
}