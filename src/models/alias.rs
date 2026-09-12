use crate::utils::TrackError;
use serde::Serialize;
use std::fmt;
use std::ops::Deref;

const MAX_ALIAS_LEN: usize = 50;
const RESERVED_ALIASES: &[&str] = &[
    "new", "list", "current", "status", "switch", "archive", "sync", "todo", "scrap", "link",
    "repo", "desc", "ticket", "alias", "help", "webui",
];

/// Human-assigned task alias (`track switch oauth-fix`).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
#[serde(transparent)]
pub struct TaskAlias(String);

impl TaskAlias {
    /// Parses an alias at a write boundary.
    pub fn parse(raw: &str) -> Result<Self, TrackError> {
        if raw.is_empty() || raw.len() > MAX_ALIAS_LEN {
            return Err(TrackError::InvalidAlias(
                "Alias must be between 1 and 50 characters".to_string(),
            ));
        }

        if !raw
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
        {
            return Err(TrackError::InvalidAlias(
                "Alias can only contain alphanumeric characters, hyphens, and underscores"
                    .to_string(),
            ));
        }

        if RESERVED_ALIASES.contains(&raw.to_ascii_lowercase().as_str()) {
            return Err(TrackError::InvalidAlias(format!(
                "Alias '{raw}' is a reserved word"
            )));
        }

        Ok(Self(raw.to_string()))
    }

    /// Reconstructs an alias already stored in SQLite.
    pub fn from_stored(raw: String) -> Self {
        Self(raw)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for TaskAlias {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl AsRef<str> for TaskAlias {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl Deref for TaskAlias {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        self.as_str()
    }
}

impl PartialEq<str> for TaskAlias {
    fn eq(&self, other: &str) -> bool {
        self.0 == other
    }
}

impl PartialEq<&str> for TaskAlias {
    fn eq(&self, other: &&str) -> bool {
        self.0 == *other
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_accepts_valid_aliases() {
        assert!(TaskAlias::parse("valid-alias").is_ok());
        assert!(TaskAlias::parse("valid_alias").is_ok());
        assert!(TaskAlias::parse("ValidAlias123").is_ok());
        assert!(TaskAlias::parse(&"a".repeat(50)).is_ok());
    }

    #[test]
    fn parse_rejects_invalid_aliases() {
        assert!(TaskAlias::parse("").is_err());
        assert!(TaskAlias::parse(&"a".repeat(51)).is_err());
        assert!(TaskAlias::parse("invalid alias").is_err());
        assert!(TaskAlias::parse("invalid@alias").is_err());
        assert!(TaskAlias::parse("invalid.alias").is_err());
        assert!(TaskAlias::parse("new").is_err());
        assert!(TaskAlias::parse("list").is_err());
        assert!(TaskAlias::parse("status").is_err());
        assert!(TaskAlias::parse("NEW").is_err());
    }
}
