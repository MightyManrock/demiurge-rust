#![allow(dead_code)]

mod common;
mod bio;
mod data;

use uuid::Uuid;

fn main() {

    let my_species = bio::Species {
        id: Uuid::new_v4(),
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
        repro_profile: bio::ReproductionProfile {
            sex_types: vec![
                bio::SexType {
                    name: String::from("Male"),
                    symbol: Some(String::from("♂")),
                    reproductive_role: Some(vec![bio::ReproductiveRole::Contributor])
                },
                bio::SexType {
                    name: String::from("Female"),
                    symbol: Some(String::from("♀")),
                    reproductive_role: Some(vec![bio::ReproductiveRole::Receiver])
                },
            ],
            is_sexual: true,
        },
        temp_range: Some(common::Range { min: 20.0, max: 25.0 }),
        press_range: Some(common::Range { min: 80.0, max: 120.0 }),
        grav_range: Some(common::Range { min: 0.35, max: 2.25 })
    };

    let species_list_to_write = vec![my_species];

    data::write_db("core.db", species_list_to_write);
    
    let species_list_to_read = data::read_db("core.db");

    for species in species_list_to_read {
        println!("{:#?}", species);
    }

}