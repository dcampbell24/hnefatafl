use serde::{Deserialize, Serialize};

pub const MAX_VOLUME: u32 = 8;

#[derive(Debug, Deserialize, Serialize)]
pub(crate) struct Volume(pub u32);

impl Default for Volume {
    fn default() -> Self {
        Self(4)
    }
}

impl Volume {
    pub fn volume(&self) -> f32 {
        match self.0 {
            0 => 0.25,
            1 => 0.375,
            2 => 0.5,
            3 => 0.75,
            4 => 1.0,
            5 => 1.5,
            6 => 2.0,
            7 => 3.0,
            MAX_VOLUME => 4.0,
            _ => unreachable!(),
        }
    }
}
