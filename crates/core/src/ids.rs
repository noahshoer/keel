use std::fmt;

use serde::{Deserialize, Serialize};
use ulid::Ulid;

/// A client-generated, time-sortable string identifier (ULID).
#[derive(Clone, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub struct Id(String);

impl Id {
    /// Generate a fresh ULID-based id.
    pub fn new() -> Self {
        Id(Ulid::new().to_string())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn into_string(self) -> String {
        self.0
    }
}

impl Default for Id {
    fn default() -> Self {
        Self::new()
    }
}

impl From<String> for Id {
    fn from(s: String) -> Self {
        Id(s)
    }
}

impl From<&str> for Id {
    fn from(s: &str) -> Self {
        Id(s.to_string())
    }
}

impl fmt::Display for Id {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}
