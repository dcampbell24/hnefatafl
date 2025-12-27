use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct Tournament {
    pub byes: Vec<String>,
    pub round_one: Vec<String>,
}
