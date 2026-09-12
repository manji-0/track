use crate::utils::TrackError;
use serde::Serialize;
use std::fmt;
use std::ops::Deref;

/// HTTP(S) URL stored on a task link.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
#[serde(transparent)]
pub struct HttpUrl(String);

impl HttpUrl {
    /// Parses an HTTP or HTTPS URL at a write boundary.
    pub fn parse(raw: &str) -> Result<Self, TrackError> {
        if raw.starts_with("http://") || raw.starts_with("https://") {
            Ok(Self(raw.to_string()))
        } else {
            Err(TrackError::InvalidUrl(raw.to_string()))
        }
    }

    /// Reconstructs a URL already stored in SQLite.
    pub fn from_stored(raw: String) -> Self {
        Self(raw)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for HttpUrl {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl AsRef<str> for HttpUrl {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl Deref for HttpUrl {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        self.as_str()
    }
}

impl PartialEq<str> for HttpUrl {
    fn eq(&self, other: &str) -> bool {
        self.0 == other
    }
}

impl PartialEq<&str> for HttpUrl {
    fn eq(&self, other: &&str) -> bool {
        self.0 == *other
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_accepts_http_urls() {
        assert_eq!(
            HttpUrl::parse("https://example.com").unwrap().as_str(),
            "https://example.com"
        );
        assert!(HttpUrl::parse("http://example.com").is_ok());
        assert!(HttpUrl::parse("ftp://example.com").is_err());
        assert!(HttpUrl::parse("example.com").is_err());
    }
}
