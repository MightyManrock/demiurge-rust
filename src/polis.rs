use serde::{Serialize, Deserialize};
use uuid::Uuid;
use std::collections::HashMap;

use crate::dom_cult_reg::{DomainTag, ReligionTag, VirtueTag, PracticeTag};
use crate::universe::{EntityAge};

#[derive(Debug, Serialize, Deserialize)]
pub enum CosmologyTag {
    Animist,
    Ancestral,
    Polytheist,
    Monotheist,
    Dualist,
    Cyclical,
    VoidWorship,
    Nontheist,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum OntoStance {
    Reframing,
    Devotional,
    Rejection,
    Agnostic,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Orthodoxy {
    pub beliefs: HashMap<DomainTag, f32>,
    pub culture: HashMap<VirtueTag, f32>,
    pub cosmo: Vec<CosmologyTag>,
    pub onto_awareness: f32,
    pub onto_stance: Option<OntoStance>,
    pub strength: f32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Praxy {
    // pub rituals: Vec<String>,    // TO-DO: formalize rituals.
    pub imago: Option<Vec<Uuid>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AdherenceSummary {
    pub pop_count: u32,
    pub pop_size: f32,
    pub mortal_count: u32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Religion {
    pub id: Uuid,
    pub name: String,
    pub age: EntityAge,
    pub founding_world_id: Uuid,
    pub founding_civ_id: Option<Uuid>,
    pub ortho: Orthodoxy,
    pub praxy: Praxy,
    pub predecessor_id: Option<Vec<Uuid>>,
    pub adherence: AdherenceSummary,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Gov {
    pub id: Uuid,
    pub name: String,
    pub age: EntityAge,
    pub home_world_id: Uuid,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Faction {
    pub id: Uuid,
    pub name: String,
    pub age: EntityAge,
    pub home_world_id: Uuid,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Pop {
    pub id: Uuid,
    pub name: String,
    pub age: EntityAge,
    pub home_world_id: Uuid,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Civ {
    pub id: Uuid,
    pub name: String,
    pub age: EntityAge,
    pub home_world_id: Uuid,
}
