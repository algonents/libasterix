pub mod cat048;
pub mod cat062;
pub(crate) mod cursor;
pub mod write_cursor;

use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum AsterixCategory {
    Cat048,
    Cat062,
    Unknown(u8),
}

impl From<u8> for AsterixCategory {
    fn from(v: u8) -> Self {
        match v {
            0x30 => AsterixCategory::Cat048, // 48 decimal = 0x30
            0x3E => AsterixCategory::Cat062, // 62 decimal = 0x3E
            other => AsterixCategory::Unknown(other),
        }
    }
}

impl AsterixCategory {
    pub fn raw(self) -> u8 {
        match self {
            AsterixCategory::Cat048 => 0x30,
            AsterixCategory::Cat062 => 0x3E,
            AsterixCategory::Unknown(v) => v,
        }
    }
}
