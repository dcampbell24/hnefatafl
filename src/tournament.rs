use std::collections::HashSet;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct Tournament {
    pub players: HashSet<String>,
    pub date: DateTime<Utc>,
    pub tree: Option<TournamentTree>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct TournamentTree {
    pub byes: Vec<String>,
    pub round_one: Vec<String>,
    pub rounds: Vec<Option<String>>,
}
