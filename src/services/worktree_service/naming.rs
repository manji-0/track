use crate::utils::Result;
use chrono::Utc;
use std::path::Path;

pub fn determine_branch_name(
    branch: Option<&str>,
    ticket_id: Option<&str>,
    task_id: i64,
    todo_index: Option<i64>,
) -> Result<String> {
    match (branch, ticket_id, todo_index) {
        (Some(b), Some(t), _) => Ok(format!("{}/{}", t, b)),
        (Some(b), None, _) => Ok(b.to_string()),
        (None, Some(t), Some(todo)) => Ok(format!("{}-todo-{}", t, todo)),
        (None, None, Some(todo)) => Ok(format!("task-{}-todo-{}", task_id, todo)),
        (None, Some(t), None) => Ok(format!("task/{}", t)),
        (None, None, None) => {
            let timestamp = Utc::now().timestamp();
            Ok(format!("task-{}-{}", task_id, timestamp))
        }
    }
}

pub fn task_bookmark_name(task_id: i64, ticket_id: Option<&str>) -> String {
    if let Some(ticket) = ticket_id {
        format!("task/{}", ticket)
    } else {
        format!("task/task-{}", task_id)
    }
}

pub fn determine_worktree_path(repo_path: &str, branch: &str) -> Result<String> {
    let sanitized_branch = branch.replace(['/', '\\'], "_");
    let worktree_path = Path::new(repo_path).join(sanitized_branch);
    Ok(worktree_path.to_string_lossy().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn determine_branch_name_with_explicit_branch_and_ticket() {
        let result = determine_branch_name(Some("feature-x"), Some("PROJ-123"), 1, None).unwrap();
        assert_eq!(result, "PROJ-123/feature-x");
    }

    #[test]
    fn determine_branch_name_with_explicit_branch_only() {
        let result = determine_branch_name(Some("feature-y"), None, 1, None).unwrap();
        assert_eq!(result, "feature-y");
    }

    #[test]
    fn determine_branch_name_with_ticket_and_todo() {
        let result = determine_branch_name(None, Some("PROJ-456"), 1, Some(5)).unwrap();
        assert_eq!(result, "PROJ-456-todo-5");
    }

    #[test]
    fn determine_branch_name_with_todo_only() {
        let result = determine_branch_name(None, None, 2, Some(7)).unwrap();
        assert_eq!(result, "task-2-todo-7");
    }

    #[test]
    fn determine_branch_name_base_with_ticket() {
        let result = determine_branch_name(None, Some("PROJ-789"), 3, None).unwrap();
        assert_eq!(result, "task/PROJ-789");
    }

    #[test]
    fn determine_branch_name_base_without_ticket() {
        let result = determine_branch_name(None, None, 4, None).unwrap();
        assert!(result.starts_with("task-4-"));
    }

    #[test]
    fn determine_worktree_path_sanitizes_slashes() {
        let repo_path = if cfg!(windows) {
            "C:\\path\\to\\repo"
        } else {
            "/path/to/repo"
        };
        let result = determine_worktree_path(repo_path, "feature/test").unwrap();
        let expected = Path::new(repo_path)
            .join("feature_test")
            .to_string_lossy()
            .to_string();
        assert_eq!(result, expected);
    }
}
