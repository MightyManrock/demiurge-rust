use serde::{Serialize, Deserialize};
use std::fmt;
// use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize, Hash, Eq, PartialEq)]
pub enum DomainTag {
    Truth,
    Order,
    Silence,
    Change,
    Conflict,
    Passion,
    Persistence,
    Void,
    Growth,
    Decay,
    Memory,
    Sacrifice,
    Light,
    Mastery,
    Secrecy,
    Community,
}

#[derive(Debug, Serialize, Deserialize, Hash, Eq, PartialEq)]
pub enum ReligionTag {
    AncestorWorship,
    Animism,
    LuminaryWorship,
    DemiurgeWorship,
    Nontheism,
    Maltheism,
    VoidWorship,
}

#[derive(Debug, Serialize, Deserialize, Hash, Eq, PartialEq)]
pub enum VirtueTrait {
    // Social Structure
    Hierarchy,
    Meritocracy,
    Solidarity,
    Sedentism,
    Xenophilia,
    // Intellectual and Epistemic
    Erudition,
    Pragmatism,
    Tradition,
    // Character and Virtue
    Honor,
    Sincerity,
    Humility,
    Wit,
    Patience,
    Adaptability,
    // Material and Social Orientation
    Moderation,
    Prosperity,
}

impl VirtueTrait {
    fn name(&self) -> &'static str {
        match self {
            Self::Hierarchy => "Hierarchy",
            Self::Meritocracy => "Meritocracy",
            Self::Solidarity => "Solidarity",
            Self::Sedentism => "Sedentism",
            Self::Xenophilia => "Xenophilia",
            Self::Erudition => "Erudition",
            Self::Pragmatism => "Pragmatism",
            Self::Tradition => "Tradition",
            Self::Honor => "Honor",
            Self::Sincerity => "Sincerity",
            Self::Humility => "Humility",
            Self::Wit => "Wit",
            Self::Patience => "Patience",
            Self::Adaptability => "Adaptability",
            Self::Moderation => "Moderation",
            Self::Prosperity => "Prosperity",
        }
    }

    fn antonym(&self) -> &'static str {
        match self {
            Self::Hierarchy => "Egalitarianism",
            Self::Meritocracy => "Equity",
            Self::Solidarity => "Autonomy",
            Self::Sedentism => "Nomadism",
            Self::Xenophilia => "Xenophobia",
            Self::Erudition => "Folk Wisdom",
            Self::Pragmatism => "Idealism",
            Self::Tradition => "Innovation",
            Self::Honor => "Opportunism",
            Self::Sincerity => "Stoicism",
            Self::Humility => "Prowess",
            Self::Wit => "Solemnity",
            Self::Patience => "Tenacity",
            Self::Adaptability => "Constancy",
            Self::Moderation => "Indulgence",
            Self::Prosperity => "Charity",
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Hash, Eq, PartialEq)]
pub enum VirtueTag {
    Positive(VirtueTrait),
    Negative(VirtueTrait),
}

impl fmt::Display for VirtueTag {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Positive(t) => write!(f, "{}", t.name().to_lowercase()),
            Self::Negative(t) => write!(f, "{}", t.antonym().to_lowercase()),
        }
    }
}


#[derive(Debug, Serialize, Deserialize, Hash, Eq, PartialEq)]
pub enum PracticeTag {
    // Arts and Performance
    Music,
    Dance,
    VisualArt,
    Drama,
    Literature,
    Poetry,
    // Craft
    Crafts,
    Culinary,
    // Physical
    Athletics,
    Combat,
    // Ceremonial and Social
    Ritual,
    Revelry,
}