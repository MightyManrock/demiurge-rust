use serde::{Serialize, Deserialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Range<T> {
    pub min: T,
    pub max: T,
}
