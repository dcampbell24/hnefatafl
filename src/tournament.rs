use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct Tournament {
    pub tree: Vec<Option<String>>,
}

impl Tournament {
    #[must_use]
    pub fn new(vec: Vec<Option<String>>) -> Self {
        Tournament { tree: vec }
    }
}
