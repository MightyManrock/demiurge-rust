use serde::{Serialize, Deserialize};

#[derive(Debug)]
#[derive(Serialize, Deserialize)]
pub struct Range<T> {
    pub min: T,
    pub max: T,
}