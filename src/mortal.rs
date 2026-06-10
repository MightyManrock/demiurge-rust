use serde::{Serialize, Deserialize};
use uuid::Uuid;
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize)]
pub enum DivineRole {
    Proxius,
    Herald,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum DivineStatus {
    Active,
    Dormant,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DivineSpark {                // More "divine relational" fields to be on
    pub role: Option<DivineRole>,       // MortalState, as well as things like divine
    pub status: Option<DivineStatus>,   // powers unlocked, Imāginēs know (if applicable),
    pub appointed_tick: Option<u32>,    // etc.
}

#[derive(Debug, Serialize, Deserialize)]
pub enum GroupRole {
    Member,
    Leader,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FactionAffiliation {
    pub faction_id: Uuid,
    pub role: GroupRole,
    pub prominence: f32,
    pub joined_tick: u32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BandAffiliation {
    pub band_id: Uuid,
    pub role: GroupRole,
    pub embedded: bool,
    pub joined_tick: u32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MortalIdentity {
//  pub beliefs: HashMap<Belief, f32>,  // Belief and Culture enums to be shared
//  pub culture: HashMap<Culture, f32>, // by all entities (Civs, Pops, etc.)
//  pub personality: HashMap<Personality, f32>,
//  pub gender: MortalGender,
}

// Gender will likely end up drawing on aspects of the parent
// civilization. Might also encompass sexuality if that is ever
// relevant to the simulation.

#[derive(Debug, Serialize, Deserialize, Hash, Eq, PartialEq)]
pub enum Skill {
    // Material & Occupational
    Trade,
    Craft,
    Labor,
    Navigation,
    Engineering,
    Medicine,
    Combat,
    // Social
        // Pop-facing
        Rhetoric,
        Ritual,
        Performance,
        // Faction-facing
        Leadership,
        Diplomacy,
    // Covert
    Stealth,
    // Knowledge
    Scholarship,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MortalAge {
    pub birthday_billions: Option<u32>,     // `bio_age` as a property derived from
    pub birthday_millions: Option<u32>,     // in-game date - birthday`? `chrono_age`
    pub birthday_thousands: Option<u32>,    // derived either as copy of `bio_age` or
    pub birthday_years: u32,                // calculated with div_spark.appointed_tick
    pub birthday_month: u32,                // considered?
    pub birthday_day: u32,
    pub bio_age: u32,               
    pub chrono_age: Option<u32>,    
}                                   

#[derive(Debug, Serialize, Deserialize)]
pub struct Mortal {
    pub id: Uuid,
//  pub name: MortalName,   // TO-DO: create a MortalName struct that references
//  pub title: MortalTitle, // naming conventions of a Civ, as well as a
    pub species_id: Uuid,   // MortalTitle struct that has a bit of logic to it.
//  pub sex: MortalSex,     // MortalSex will draw from the parent species' defined
    pub civ_id: Uuid,       // qualities. (Not all are male/female or even reproduce
    pub age: MortalAge,     // sexually, so it will necessarily get complicated.)
    pub home_loc_id: Uuid,
    pub current_loc_id: Uuid,
    pub orig_pop_id: Uuid,
    pub milieu_pop_id: Uuid,
    pub faction_affs: Option<Vec<FactionAffiliation>>,
    pub band_aff: Option<BandAffiliation>,
    pub div_spark: DivineSpark,
    pub identity: MortalIdentity,
//  pub occupation: Occupation, // This will probably come from an Enum and default to orig_pop's occupation.
//  pub status: String,                 // Merge in a struct with title or something else?
    pub skills: HashMap<Skill, f32>,
//  pub agent: MortalAgent,             // This will be defined in agent defs.
//  pub visibility: EntityVisibility,   // This will be defined for entities generally.
    pub pinned: bool,
}