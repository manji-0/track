use crate::models::Task;

/// Derives the jj-task slug for a track task.
///
/// Priority: alias → ticket_id (sanitized) → `task-{id}`.
/// Matches [agent-skill-jj](https://github.com/manji-0/agent-skill-jj) conventions.
pub fn jj_slug(task: &Task) -> String {
    if let Some(alias) = task.alias.as_deref() {
        return sanitize_jj_slug(alias);
    }
    if let Some(ticket) = task.ticket_id.as_deref() {
        return sanitize_jj_slug(ticket);
    }
    format!("task-{}", task.id)
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
    use crate::models::TaskStatus;
    use chrono::Utc;

    fn sample_task(id: i64, ticket: Option<&str>, alias: Option<&str>) -> Task {
        Task {
            id,
            name: "Test".to_string(),
            description: None,
            status: TaskStatus::Active,
            ticket_id: ticket.map(str::to_string),
            ticket_url: None,
            alias: alias.map(str::to_string),
            is_today_task: false,
            created_at: Utc::now(),
        }
    }

    #[test]
    fn jj_slug_prefers_alias() {
        let task = sample_task(1, Some("PROJ-123"), Some("oauth-fix"));
        assert_eq!(jj_slug(&task), "oauth-fix");
    }

    #[test]
    fn jj_slug_uses_ticket_when_no_alias() {
        let task = sample_task(2, Some("PROJ-456"), None);
        assert_eq!(jj_slug(&task), "proj-456");
    }

    #[test]
    fn jj_slug_falls_back_to_task_id() {
        let task = sample_task(7, None, None);
        assert_eq!(jj_slug(&task), "task-7");
    }

    #[test]
    fn sanitize_jj_slug_normalizes_ticket() {
        assert_eq!(sanitize_jj_slug("BUG_999"), "bug-999");
        assert_eq!(sanitize_jj_slug("  Feature/X  "), "feature-x");
    }
}
