use serde::{Deserialize, Serialize};

use crate::glicko::Rating;

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub enum Size {
    Tiny,
    TinyWide,
    #[default]
    Small,
    Medium,
    Large,
    Giant,
}

#[derive(Clone, Debug)]
pub enum Move {
    From,
    To,
    Revert,
    None,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LoggedIn {
    No,
    None,
    Yes,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub enum Theme {
    #[default]
    Dark,
    Light,
    Tol,
}

#[derive(Clone, Debug)]
pub struct User {
    pub name: String,
    pub wins: String,
    pub losses: String,
    pub draws: String,
    pub rating: Rating,
    pub logged_in: LoggedIn,
}
