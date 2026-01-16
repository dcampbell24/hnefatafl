#[cfg(feature = "server")]
use lettre::message::Mailbox;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Deserialize, Hash, PartialEq, Eq, Serialize)]
pub struct Email {
    #[serde(default)]
    pub address: String,
    #[serde(default)]
    pub code: Option<u32>,
    #[serde(default)]
    pub username: String,
    #[serde(default)]
    pub verified: bool,
}

#[cfg(feature = "server")]
impl Email {
    #[must_use]
    pub fn to_mailbox(&self) -> Option<Mailbox> {
        Some(Mailbox::new(
            Some(self.username.clone()),
            self.address.parse().ok()?,
        ))
    }

    #[must_use]
    pub fn tx(&self) -> String {
        // Note: We use a FIGURE SPACE to separate the username from the address so
        // .split_ascii_whitespace() does not treat it as a space.
        format!("{} <{}>", self.username, self.address)
    }
}
