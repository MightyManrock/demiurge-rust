use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Range<T> {
    pub min: T,
    pub max: T,
}
