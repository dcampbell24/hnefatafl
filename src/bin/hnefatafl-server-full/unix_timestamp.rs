use chrono::Local;
use serde::{Deserialize, Serialize};

/// Non-leap seconds since January 1, 1970 0:00:00 UTC.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct UnixTimestamp(pub i64);

impl Default for UnixTimestamp {
    fn default() -> Self {
        Self(Local::now().to_utc().timestamp())
    }
}
