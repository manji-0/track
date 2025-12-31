use rusqlite::{params, OptionalExtension};
use chrono::Utc;
use std::path::Path;
use std::process::Command;
use crate::db::Database;
use crate::models::{GitItem, RepoLink};
use crate::utils::{Result, TrackError};

pub struct WorktreeService<'a> {
    db: &'a Database,
}

impl<'a> WorktreeService<'a> {
    pub fn new(db: &'a Database) -> Self {
        Self { db }
    }

    pub fn add_worktree(
        &self,
        task_id: i64,
        repo_path: &str,
        branch: Option<&str>,
        ticket_id: Option<&str>,
        todo_id: Option<i64>,
        is_base: bool,
    ) -> Result<GitItem> {
        // Verify it's a git repository
        if !self.is_git_repository(repo_path)? {
            return Err(TrackError::NotGitRepository(repo_path.to_string()));
        }

        // Determine branch name
        let branch_name = self.determine_branch_name(branch, ticket_id, task_id, todo_id)?;

        // Check if branch already exists
        if self.branch_exists(repo_path, &branch_name)? {
            return Err(TrackError::BranchExists(branch_name));
        }

        // Determine worktree path
        let worktree_path = self.determine_worktree_path(repo_path, &branch_name)?;

        // Create worktree
        self.create_git_worktree(repo_path, &worktree_path, &branch_name)?;

        // Register in database
        let now = Utc::now().to_rfc3339();
        let conn = self.db.get_connection();

        conn.execute(
            "INSERT INTO git_items (task_id, path, branch, base_repo, status, created_at, todo_id, is_base) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![task_id, worktree_path, branch_name, repo_path, "active", now, todo_id, is_base as i32],
        )?;

        let git_item_id = conn.last_insert_rowid();
        self.get_git_item(git_item_id)
    }

    pub fn get_git_item(&self, git_item_id: i64) -> Result<GitItem> {
        let conn = self.db.get_connection();
        let mut stmt = conn.prepare(
            "SELECT id, task_id, path, branch, base_repo, status, created_at, todo_id, is_base FROM git_items WHERE id = ?1"
        )?;

        let git_item = stmt.query_row(params![git_item_id], |row| {
            let is_base: i32 = row.get(8).unwrap_or(0);
            Ok(GitItem {
                id: row.get(0)?,
                task_id: row.get(1)?,
                path: row.get(2)?,
                branch: row.get(3)?,
                base_repo: row.get(4)?,
                status: row.get(5)?,
                created_at: row.get::<_, String>(6)?.parse().unwrap(),
                todo_id: row.get(7)?,
                is_base: is_base != 0,
            })
        }).map_err(|_| TrackError::WorktreeNotFound(git_item_id))?;

        Ok(git_item)
    }

    pub fn list_worktrees(&self, task_id: i64) -> Result<Vec<GitItem>> {
        let conn = self.db.get_connection();
        let mut stmt = conn.prepare(
            "SELECT id, task_id, path, branch, base_repo, status, created_at, todo_id, is_base FROM git_items WHERE task_id = ?1 ORDER BY created_at ASC"
        )?;

        let git_items = stmt.query_map(params![task_id], |row| {
            let is_base: i32 = row.get(8).unwrap_or(0);
            Ok(GitItem {
                id: row.get(0)?,
                task_id: row.get(1)?,
                path: row.get(2)?,
                branch: row.get(3)?,
                base_repo: row.get(4)?,
                status: row.get(5)?,
                created_at: row.get::<_, String>(6)?.parse().unwrap(),
                todo_id: row.get(7)?,
                is_base: is_base != 0,
            })
        })?
        .collect::<std::result::Result<Vec<_>, _>>()?;

        Ok(git_items)
    }

    pub fn add_repo_link(&self, git_item_id: i64, url: &str) -> Result<RepoLink> {
        let kind = self.determine_link_kind(url);
        let now = Utc::now().to_rfc3339();
        let conn = self.db.get_connection();

        conn.execute(
            "INSERT INTO repo_links (git_item_id, url, kind, created_at) VALUES (?1, ?2, ?3, ?4)",
            params![git_item_id, url, kind, now],
        )?;

        let link_id = conn.last_insert_rowid();
        self.get_repo_link(link_id)
    }

    pub fn get_repo_link(&self, link_id: i64) -> Result<RepoLink> {
        let conn = self.db.get_connection();
        let mut stmt = conn.prepare(
            "SELECT id, git_item_id, url, kind, created_at FROM repo_links WHERE id = ?1"
        )?;

        let repo_link = stmt.query_row(params![link_id], |row| {
            Ok(RepoLink {
                id: row.get(0)?,
                git_item_id: row.get(1)?,
                url: row.get(2)?,
                kind: row.get(3)?,
                created_at: row.get::<_, String>(4)?.parse().unwrap(),
            })
        })?;

        Ok(repo_link)
    }

    pub fn list_repo_links(&self, git_item_id: i64) -> Result<Vec<RepoLink>> {
        let conn = self.db.get_connection();
        let mut stmt = conn.prepare(
            "SELECT id, git_item_id, url, kind, created_at FROM repo_links WHERE git_item_id = ?1 ORDER BY created_at ASC"
        )?;

        let repo_links = stmt.query_map(params![git_item_id], |row| {
            Ok(RepoLink {
                id: row.get(0)?,
                git_item_id: row.get(1)?,
                url: row.get(2)?,
                kind: row.get(3)?,
                created_at: row.get::<_, String>(4)?.parse().unwrap(),
            })
        })?
        .collect::<std::result::Result<Vec<_>, _>>()?;

        Ok(repo_links)
    }

    pub fn remove_worktree(&self, git_item_id: i64, keep_files: bool) -> Result<()> {
        let git_item = self.get_git_item(git_item_id)?;

        if !keep_files {
            // Remove git worktree
            if let Some(base_repo) = &git_item.base_repo {
                self.remove_git_worktree(base_repo, &git_item.path)?;
            }
        }

        // Remove from database
        let conn = self.db.get_connection();
        conn.execute("DELETE FROM git_items WHERE id = ?1", params![git_item_id])?;

        Ok(())
    }

    fn is_git_repository(&self, path: &str) -> Result<bool> {
        let output = Command::new("git")
            .args(&["-C", path, "rev-parse", "--git-dir"])
            .output()?;

        Ok(output.status.success())
    }

    fn branch_exists(&self, repo_path: &str, branch: &str) -> Result<bool> {
        let output = Command::new("git")
            .args(&["-C", repo_path, "rev-parse", "--verify", branch])
            .output()?;

        Ok(output.status.success())
    }

    fn determine_branch_name(
        &self,
        branch: Option<&str>,
        ticket_id: Option<&str>,
        task_id: i64,
        todo_id: Option<i64>,
    ) -> Result<String> {
        match (branch, ticket_id, todo_id) {
            // If branch is explicitly specified, use it (with ticket prefix if available)
            (Some(b), Some(t), _) => Ok(format!("{}/{}", t, b)),
            (Some(b), None, _) => Ok(b.to_string()),

            // If todo_id is present
            (None, Some(t), Some(todo)) => Ok(format!("{}-todo-{}", t, todo)),
            (None, None, Some(todo)) => Ok(format!("task-{}-todo-{}", task_id, todo)),

            // Base worktree (no todo_id)
            (None, Some(t), None) => Ok(format!("task/{}", t)),
            (None, None, None) => {
                let timestamp = Utc::now().timestamp();
                Ok(format!("task-{}-{}", task_id, timestamp))
            }
        }
    }

    fn determine_worktree_path(&self, repo_path: &str, branch: &str) -> Result<String> {
        let repo_path = Path::new(repo_path);
        let repo_name = repo_path
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or_else(|| TrackError::Other("Invalid repository path".to_string()))?;

        let parent = repo_path
            .parent()
            .ok_or_else(|| TrackError::Other("Repository has no parent directory".to_string()))?;

        let worktree_dir = parent.join(format!("{}-worktrees", repo_name));
        let worktree_path = worktree_dir.join(branch);

        Ok(worktree_path.to_string_lossy().to_string())
    }

    fn create_git_worktree(&self, repo_path: &str, worktree_path: &str, branch: &str) -> Result<()> {
        let output = Command::new("git")
            .args(&["-C", repo_path, "worktree", "add", "-b", branch, worktree_path])
            .output()?;

        if !output.status.success() {
            let error = String::from_utf8_lossy(&output.stderr);
            return Err(TrackError::Git(error.to_string()));
        }

        Ok(())
    }

    fn remove_git_worktree(&self, repo_path: &str, worktree_path: &str) -> Result<()> {
        let output = Command::new("git")
            .args(&["-C", repo_path, "worktree", "remove", worktree_path])
            .output()?;

        if !output.status.success() {
            let error = String::from_utf8_lossy(&output.stderr);
            return Err(TrackError::Git(error.to_string()));
        }

        Ok(())
    }

    fn determine_link_kind(&self, url: &str) -> String {
        if url.contains("/pull/") || url.contains("/merge_requests/") {
            "PR".to_string()
        } else if url.contains("/issues/") {
            "Issue".to_string()
        } else if url.contains("/discussions/") {
            "Discussion".to_string()
        } else {
            "Link".to_string()
        }
    }

    pub fn complete_worktree_for_todo(&self, todo_id: i64) -> Result<Option<String>> {
        let wt = match self.get_worktree_by_todo(todo_id)? {
            Some(wt) => wt,
            None => return Ok(None),
        };

        let base_wt = self.get_base_worktree(wt.task_id)?
            .ok_or_else(|| TrackError::Other("Base worktree not found. Please init a base worktree first.".to_string()))?;

        if self.has_uncommitted_changes(&wt.path)? {
            return Err(TrackError::Other(format!("Worktree {} has uncommitted changes. Please commit or stash them.", wt.path)));
        }

        self.merge_branch(&base_wt.path, &wt.branch)?;
        self.remove_worktree(wt.id, false)?;

        Ok(Some(wt.branch))
    }

    fn get_worktree_by_todo(&self, todo_id: i64) -> Result<Option<GitItem>> {
        let conn = self.db.get_connection();
        let mut stmt = conn.prepare(
            "SELECT id, task_id, path, branch, base_repo, status, created_at, todo_id, is_base FROM git_items WHERE todo_id = ?1"
        )?;

        let result = stmt.query_row(params![todo_id], |row| {
            let is_base: i32 = row.get(8).unwrap_or(0);
            Ok(GitItem {
                id: row.get(0)?,
                task_id: row.get(1)?,
                path: row.get(2)?,
                branch: row.get(3)?,
                base_repo: row.get(4)?,
                status: row.get(5)?,
                created_at: row.get::<_, String>(6)?.parse().unwrap(),
                todo_id: row.get(7)?,
                is_base: is_base != 0,
            })
        }).optional()?;

        Ok(result)
    }

    fn get_base_worktree(&self, task_id: i64) -> Result<Option<GitItem>> {
        let conn = self.db.get_connection();
        let mut stmt = conn.prepare(
            "SELECT id, task_id, path, branch, base_repo, status, created_at, todo_id, is_base FROM git_items WHERE task_id = ?1 AND is_base = 1"
        )?;

        let result = stmt.query_row(params![task_id], |row| {
            let is_base: i32 = row.get(8).unwrap_or(0);
            Ok(GitItem {
                id: row.get(0)?,
                task_id: row.get(1)?,
                path: row.get(2)?,
                branch: row.get(3)?,
                base_repo: row.get(4)?,
                status: row.get(5)?,
                created_at: row.get::<_, String>(6)?.parse().unwrap(),
                todo_id: row.get(7)?,
                is_base: is_base != 0,
            })
        }).optional()?;

        Ok(result)
    }

    fn has_uncommitted_changes(&self, path: &str) -> Result<bool> {
        let output = Command::new("git")
            .args(&["-C", path, "status", "--porcelain"])
            .output()?;
        
        Ok(!output.stdout.is_empty())
    }

    fn merge_branch(&self, target_path: &str, branch: &str) -> Result<()> {
        let output = Command::new("git")
            .args(&["-C", target_path, "merge", "--no-ff", branch])
            .output()?;

        if !output.status.success() {
            let error = String::from_utf8_lossy(&output.stderr);
            return Err(TrackError::Git(format!("Merge failed: {}", error)));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Database;

    fn setup_db() -> Database {
        Database::new_in_memory().unwrap()
    }

    #[test]
    fn test_determine_link_kind_pr() {
        let db = setup_db();
        let service = WorktreeService::new(&db);

        assert_eq!(service.determine_link_kind("https://github.com/owner/repo/pull/123"), "PR");
        assert_eq!(service.determine_link_kind("https://gitlab.com/owner/repo/merge_requests/456"), "PR");
    }

    #[test]
    fn test_determine_link_kind_issue() {
        let db = setup_db();
        let service = WorktreeService::new(&db);

        assert_eq!(service.determine_link_kind("https://github.com/owner/repo/issues/789"), "Issue");
    }

    #[test]
    fn test_determine_link_kind_discussion() {
        let db = setup_db();
        let service = WorktreeService::new(&db);

        assert_eq!(service.determine_link_kind("https://github.com/owner/repo/discussions/42"), "Discussion");
    }

    #[test]
    fn test_determine_link_kind_generic() {
        let db = setup_db();
        let service = WorktreeService::new(&db);

        assert_eq!(service.determine_link_kind("https://example.com/some/page"), "Link");
    }

    #[test]
    fn test_determine_branch_name_with_explicit_branch_and_ticket() {
        let db = setup_db();
        let service = WorktreeService::new(&db);

        let result = service.determine_branch_name(Some("feature-x"), Some("PROJ-123"), 1, None).unwrap();
        assert_eq!(result, "PROJ-123/feature-x");
    }

    #[test]
    fn test_determine_branch_name_with_explicit_branch_only() {
        let db = setup_db();
        let service = WorktreeService::new(&db);

        let result = service.determine_branch_name(Some("feature-y"), None, 1, None).unwrap();
        assert_eq!(result, "feature-y");
    }

    #[test]
    fn test_determine_branch_name_with_ticket_and_todo() {
        let db = setup_db();
        let service = WorktreeService::new(&db);

        let result = service.determine_branch_name(None, Some("PROJ-456"), 1, Some(5)).unwrap();
        assert_eq!(result, "PROJ-456-todo-5");
    }

    #[test]
    fn test_determine_branch_name_with_todo_only() {
        let db = setup_db();
        let service = WorktreeService::new(&db);

        let result = service.determine_branch_name(None, None, 2, Some(7)).unwrap();
        assert_eq!(result, "task-2-todo-7");
    }

    #[test]
    fn test_determine_branch_name_base_with_ticket() {
        let db = setup_db();
        let service = WorktreeService::new(&db);

        let result = service.determine_branch_name(None, Some("PROJ-789"), 3, None).unwrap();
        assert_eq!(result, "task/PROJ-789");
    }

    #[test]
    fn test_determine_branch_name_base_without_ticket() {
        let db = setup_db();
        let service = WorktreeService::new(&db);

        let result = service.determine_branch_name(None, None, 4, None).unwrap();
        // Should contain "task-4-" followed by timestamp
        assert!(result.starts_with("task-4-"));
    }
}

