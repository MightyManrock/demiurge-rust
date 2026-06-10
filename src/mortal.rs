use serde::{Serialize, Deserialize};
use uuid::Uuid;

#[derive(Debug)]
#[derive(Serialize, Deserialize)]
pub enum DivineRole {
    Proxius,
    Herald,
}

#[derive(Debug)]
#[derive(Serialize, Deserialize)]
pub enum DivineStatus {
    Active,
    Dormant,
}

#[derive(Debug)]
#[derive(Serialize, Deserialize)]
pub struct DivineSpark {
    pub role: Option<DivineRole>,
    pub status: Option<DivineStatus>,
    pub appointed_tick: Option<u32>,    // Divine powers unlocked?
    pub alignment: f32,                 // Imāginēs known? I think
    pub loyalty: Option<f32>,           // those might be better as
    pub last_audit_tick: Option<u32>,   // part of the MortalAgent.
}

#[derive(Debug)]
#[derive(Serialize, Deserialize)]
pub enum GroupRole {
    Member,
    Leader,
}

#[derive(Debug)]
#[derive(Serialize, Deserialize)]
pub struct FactionAffiliation {
    pub faction_id: Uuid,
    pub role: GroupRole,
    pub prominence: f32,
    pub joined_tick: u32,
}

#[derive(Debug)]
#[derive(Serialize, Deserialize)]
pub struct BandAffiliation {
    pub band_id: Uuid,
    pub role: GroupRole,
    pub embedded: bool,
    pub joined_tick: u32,
}

#[derive(Debug)]
#[derive(Serialize, Deserialize)]
pub struct MortalIdentity {
    pub beliefs: Vec<String>,       // Gender will likely end up drawing on aspects
    pub culture: Vec<String>,       // of the parent civilization. Might als encompass
    pub personality: Vec<String>,   // sexuality if that is ever relevant to the
    pub gender: String,             // simulation.
}

#[derive(Debug)]
#[derive(Serialize, Deserialize)]
pub struct MortalCapability {
    pub abilities: Vec<String>,     // Iffy on `abilities`; if implemented, maybe
    pub occupation: Vec<String>,    // this should be a struct with pre-set "ability
    pub skills: Vec<String>,        // scores," like an RPG character's?
    pub status: Vec<String>,        // I.e., social status, some special societal
}                                   // role beyond factional.

#[derive(Debug)]
#[derive(Serialize, Deserialize)]
pub struct MortalAge {
    pub birthday_billions: u32,     // `bio_age` as a property derived from
    pub birthday_millions: u32,     // in-game date - birthday`? `chrono_age`
    pub birthday_thousands: u32,    // derived either as copy of `bio_age` or
    pub birthday_years: u32,        // calculated with div_spark.appointed_tick
    pub birthday_month: u32,        // considered?
    pub birthday_day: u32,
    pub bio_age: u32,               
    pub chrono_age: Option<u32>,    
}                                   

#[derive(Debug)]                    
#[derive(Serialize, Deserialize)]
pub struct Mortal {
    pub id: Uuid,
    pub name: String,       // TO-DO: create a MortalName struct that references
    pub title: String,      // naming conventions of a Civilization, as well as a
    pub species_id: Uuid,   // MortalTitle struct that has a bit of logic to it.
    pub sex: String,        // MortalSex also may wind up being a struct, which would
    pub civ_id: Uuid,       // draw from the parent species' defined qualities. (Not
    pub age: MortalAge,     // all are male/female or even reproduce sexually, so it
    pub home_loc_id: Uuid,  // will necessarily get complicated.)
    pub current_loc_id: Uuid,
    pub orig_pop_id: Uuid,
    pub milieu_pop_id: Uuid,
    pub faction_affs: Option<Vec<FactionAffiliation>>,
    pub band_aff: Option<BandAffiliation>,
    pub div_spark: DivineSpark,
    pub identity: MortalIdentity,
    pub capability: MortalCapability,
//  pub agent: MortalAgent,             // This will be defined in agent defs.
//  pub visibility: EntityVisibility,   // This will be defined for entities generally.
    pub pinned: bool,
}