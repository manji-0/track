use crate::utils::TrackError;
use serde::Serialize;
use std::fmt;

/// External ticket identifier linked to a task (Jira `PROJ-123` or GitHub `owner/repo/123`).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
#[serde(transparent)]
pub struct TicketId(String);

impl TicketId {
    /// Parses a ticket ID at a write boundary.
    pub fn parse(raw: &str) -> Result<Self, TrackError> {
        let ticket_id = raw.trim();
        if ticket_id.contains('-') && ticket_id.chars().any(|c| c.is_ascii_uppercase()) {
            return Ok(Self(ticket_id.to_string()));
        }

        let parts: Vec<&str> = ticket_id.split('/').collect();
        if parts.len() == 3 && parts[2].chars().all(|c| c.is_ascii_digit()) {
            return Ok(Self(ticket_id.to_string()));
        }

        Err(TrackError::InvalidTicketFormat(ticket_id.to_string()))
    }

    /// Reconstructs a ticket already stored in SQLite.
    pub fn from_stored(raw: String) -> Self {
        Self(raw)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for TicketId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl AsRef<str> for TicketId {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl std::ops::Deref for TicketId {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        self.as_str()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_jira_and_github_tickets() {
        assert_eq!(TicketId::parse("PROJ-123").unwrap().as_str(), "PROJ-123");
        assert_eq!(
            TicketId::parse("owner/repo/123").unwrap().as_str(),
            "owner/repo/123"
        );
        assert!(TicketId::parse("invalid").is_err());
        assert!(TicketId::parse("proj-123").is_err());
        assert!(TicketId::parse("PROJ123").is_err());
    }
}
