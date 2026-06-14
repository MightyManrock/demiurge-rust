#![allow(dead_code)]

mod common;
mod data;
mod universe;
mod bio;
mod dom_cult_reg;
// mod mortal;
// mod polis;


use uuid::Uuid;

fn main() {

    let my_species = bio::Species {
        id: Uuid::new_v4(),
        name: Some(String::from("Human")),
        kind: bio::SpeciesKind::Named,
        origin_world_id: Uuid::new_v4(),
        sentience: Some(bio::SpeciesSentience::Sapient),
        basis: bio::LifeBasis::Carbon,
        solvent: {
            bio::Solvent {
                liquid: universe::LiquidTag::Water,
                access_range: Some(common::Range { min: 0.02, max: 0.75}),
                humidity_range: Some(common::Range { min: 0.15, max: 0.85}),
            }
        },  
        atmo_aff: vec![
            bio::AtmosphereAffinity {
                tag: Some(universe::AtmosphereTag::Oxygen),
                threshold: Some(common::Range { min: 0.19, max: 0.23 }),
                relationship: bio::AtmosphereRelationship::Required,
                medium: bio::RespirationMedium::Gas,
            },
            bio::AtmosphereAffinity {
                tag: Some(universe::AtmosphereTag::CarbonMonoxide),
                threshold: Some(common::Range { min: 0.002, max: 1.0 }),
                relationship: bio::AtmosphereRelationship::Toxic,
                medium: bio::RespirationMedium::Gas,
            },
            bio::AtmosphereAffinity {
                tag: Some(universe::AtmosphereTag::Methane),
                threshold: Some(common::Range { min: 0.05, max: 1.0 }),
                relationship: bio::AtmosphereRelationship::Fatal,
                medium: bio::RespirationMedium::Gas,
            },
            bio::AtmosphereAffinity {
                tag: Some(universe::AtmosphereTag::CarbonDioxide),
                threshold: Some(common::Range { min: 0.0, max: 0.01 }),
                relationship: bio::AtmosphereRelationship::Tolerated,
                medium: bio::RespirationMedium::Gas,
            },
            bio::AtmosphereAffinity {
                tag: None,
                threshold: None,
                relationship: bio::AtmosphereRelationship::Fatal,
                medium: bio::RespirationMedium::Liquid,
            },
            bio::AtmosphereAffinity {
                tag: None,
                threshold: None,
                relationship: bio::AtmosphereRelationship::Fatal,
                medium: bio::RespirationMedium::Solid,
            },
            bio::AtmosphereAffinity {
                tag: None,
                threshold: None,
                relationship: bio::AtmosphereRelationship::Fatal,
                medium: bio::RespirationMedium::Vacuum,
            },
        ],
        food_tag: vec![
            bio::FoodTag::Herbivorous,
            bio::FoodTag::Carnivorous,
        ],
        repro_profile: bio::ReproductionProfile {
            sex_kinds: vec![
                bio::SexKind {
                    name: String::from("Male"),
                    symbol: Some(String::from("♂")),
                    reproductive_role: Some(vec![bio::ReproductiveRole::Contributor])
                },
                bio::SexKind {
                    name: String::from("Female"),
                    symbol: Some(String::from("♀")),
                    reproductive_role: Some(vec![bio::ReproductiveRole::Receiver])
                },
            ],
            repro_kind: vec![bio::ReproductionKind::Sexual],
            repro_method: Some(bio::ReproductiveMethod::Viviparity),
        },
        lifespan: Some(common::Range {min: 65, max: 110}),
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
