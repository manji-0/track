use crate::models::Task;
use serde::Serialize;
use std::fmt;
use std::ops::Deref;

/// jj-task slug derived from a track task (`alias` → `ticket_id` → `task-{id}`).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
#[serde(transparent)]
pub struct JjSlug(String);

impl JjSlug {
    fn from_sanitized(value: String) -> Self {
        Self(value)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Deref for JjSlug {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl AsRef<str> for JjSlug {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl fmt::Display for JjSlug {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl From<JjSlug> for String {
    fn from(slug: JjSlug) -> Self {
        slug.0
    }
}

impl PartialEq<str> for JjSlug {
    fn eq(&self, other: &str) -> bool {
        self.0 == other
    }
}

impl PartialEq<&str> for JjSlug {
    fn eq(&self, other: &&str) -> bool {
        self.0 == *other
    }
}

/// Derives the jj-task slug for a track task.
///
/// Priority: alias → ticket_id (sanitized) → `task-{id}`.
/// Matches [agent-skill-jj](https://github.com/manji-0/agent-skill-jj) conventions.
pub fn jj_slug(task: &Task) -> JjSlug {
    if let Some(alias) = task.alias.as_deref() {
        return JjSlug::from_sanitized(sanitize_jj_slug(alias));
    }
    if let Some(ticket) = task.ticket_id.as_deref() {
        return JjSlug::from_sanitized(sanitize_jj_slug(ticket));
    }
    JjSlug::from_sanitized(format!("task-{}", task.id))
}

/// Normalizes a string into a jj-task-compatible slug (lowercase, hyphen-separated).
pub fn sanitize_jj_slug(input: &str) -> String {
    let mut slug = String::new();
    let mut prev_hyphen = false;

    for ch in input.trim().to_ascii_lowercase().chars() {
        if ch.is_ascii_alphanumeric() {
            slug.push(ch);
            prev_hyphen = false;
        } else if !prev_hyphen && !slug.is_empty() {
            slug.push('-');
            prev_hyphen = true;
        }
    }

    let slug = slug.trim_matches('-').to_string();
    if slug.is_empty() {
        "task".to_string()
    } else {
        slug
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{Task, TaskId, TaskStatus, TicketId};
    use chrono::Utc;

    fn sample_task(id: i64, ticket: Option<&str>, alias: Option<&str>) -> Task {
        Task {
            id: TaskId::from_i64(id),
            name: "Test".to_string(),
            description: None,
            status: TaskStatus::Active,
            ticket_id: ticket.map(|t| TicketId::from_stored(t.to_string())),
            ticket_url: None,
            alias: alias.map(str::to_string),
            is_today_task: false,
            created_at: Utc::now(),
        }
    }

    #[test]
    fn jj_slug_prefers_alias() {
        let task = sample_task(1, Some("PROJ-123"), Some("oauth-fix"));
        assert_eq!(jj_slug(&task).as_str(), "oauth-fix");
    }

    #[test]
    fn jj_slug_uses_ticket_when_no_alias() {
        let task = sample_task(2, Some("PROJ-456"), None);
        assert_eq!(jj_slug(&task).as_str(), "proj-456");
    }

    #[test]
    fn jj_slug_falls_back_to_task_id() {
        let task = sample_task(7, None, None);
        assert_eq!(jj_slug(&task).as_str(), "task-7");
    }

    #[test]
    fn sanitize_jj_slug_normalizes_ticket() {
        assert_eq!(sanitize_jj_slug("BUG_999"), "bug-999");
        assert_eq!(sanitize_jj_slug("  Feature/X  "), "feature-x");
    }
}
