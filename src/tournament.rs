use core::fmt;
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
    pub rounds: Vec<Vec<Status>>,
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct Player {
    pub name: String,
    pub rating: f64,
}

impl fmt::Display for Player {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {:0}", self.name, self.rating)
    }
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct Status {
    pub processed: bool,
    pub status: StatusEnum,
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub enum StatusEnum {
    Lost(Player),
    #[default]
    None,
    Ready(Player),
    Waiting,
    Won(Player),
}
