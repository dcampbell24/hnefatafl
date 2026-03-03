use serde::{Deserialize, Serialize};

pub const MAX_VOLUME: u32 = 4;

#[derive(Debug, Deserialize, Serialize)]
pub(crate) struct Volume(pub u32);

impl Default for Volume {
    fn default() -> Self {
        Self(2)
    }
}

impl Volume {
    pub fn volume(&self) -> f32 {
        match self.0 {
            0 => 0.25,
            1 => 0.5,
            2 => 1.0,
            3 => 2.0,
            MAX_VOLUME => 4.0,
            _ => unreachable!(),
        }
    }
}
