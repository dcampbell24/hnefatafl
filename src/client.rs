use serde::{Deserialize, Serialize};

use crate::glicko::Rating;

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub enum Size {
    Tiny,
    #[default]
    Small,
    Medium,
    Large,
    Giant,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub enum Theme {
    #[default]
    Dark,
    Light,
}

#[derive(Clone, Debug)]
pub struct User {
    pub name: String,
    pub wins: String,
    pub losses: String,
    pub draws: String,
    pub rating: Rating,
    pub logged_in: bool,
}
