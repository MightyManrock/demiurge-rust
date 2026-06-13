use serde::{Serialize, Deserialize};
use uuid::Uuid;
use std::collections::HashMap;

use crate::dom_cult_reg::{DomainTag, ReligionTag, VirtueTag, PracticeTag};
use crate::universe::{EntityAge};

#[derive(Debug, Serialize, Deserialize)]
pub struct Religion {
    pub id: Uuid,
    pub name: String,
    pub age: EntityAge,
    pub home_world_id: Uuid,
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
