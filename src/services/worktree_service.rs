use crate::db::Database;
use crate::models::{GitItem, RepoLink};
use crate::utils::{Result, TrackError};
use chrono::Utc;
use rusqlite::{params, OptionalExtension};
use std::path::Path;
use std::process::Command;

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

        // Fetch todo_index if todo_id is present
        let todo_index = if let Some(t_id) = todo_id {
            let conn = self.db.get_connection();
            let idx: i64 = conn
                .query_row(
                    "SELECT task_index FROM todos WHERE id = ?1",
                    params![t_id],
                    |row| row.get(0),
                )
                .map_err(|e| match e {
                    rusqlite::Error::QueryReturnedNoRows => TrackError::TodoNotFound(t_id),
                    _ => TrackError::Database(e),
                })?;
            Some(idx)
        } else {
            None
        };

        // Determine branch name
        let branch_name = self.determine_branch_name(branch, ticket_id, task_id, todo_index)?;

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

        let git_item = stmt
            .query_row(params![git_item_id], |row| {
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
            })
            .map_err(|_| TrackError::WorktreeNotFound(git_item_id))?;

        Ok(git_item)
    }

    pub fn list_worktrees(&self, task_id: i64) -> Result<Vec<GitItem>> {
        let conn = self.db.get_connection();
        let mut stmt = conn.prepare(
            "SELECT id, task_id, path, branch, base_repo, status, created_at, todo_id, is_base FROM git_items WHERE task_id = ?1 ORDER BY created_at ASC"
        )?;

        let git_items = stmt
            .query_map(params![task_id], |row| {
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

    pub fn list_repo_links(&self, git_item_id: i64) -> Result<Vec<RepoLink>> {
        let conn = self.db.get_connection();
        let mut stmt = conn.prepare(
            "SELECT id, git_item_id, url, kind, created_at FROM repo_links WHERE git_item_id = ?1 ORDER BY created_at ASC"
        )?;

        let repo_links = stmt
            .query_map(params![git_item_id], |row| {
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
            .args(["-C", path, "rev-parse", "--git-dir"])
            .output()?;

        Ok(output.status.success())
    }

    fn branch_exists(&self, repo_path: &str, branch: &str) -> Result<bool> {
        let output = Command::new("git")
            .args(["-C", repo_path, "rev-parse", "--verify", branch])
            .output()?;

        Ok(output.status.success())
    }

    fn determine_branch_name(
        &self,
        branch: Option<&str>,
        ticket_id: Option<&str>,
        task_id: i64,
        todo_index: Option<i64>,
    ) -> Result<String> {
        match (branch, ticket_id, todo_index) {
            // If branch is explicitly specified, use it (with ticket prefix if available)
            (Some(b), Some(t), _) => Ok(format!("{}/{}", t, b)),
            (Some(b), None, _) => Ok(b.to_string()),

            // If todo_index is present
            (None, Some(t), Some(todo)) => Ok(format!("{}-todo-{}", t, todo)),
            (None, None, Some(todo)) => Ok(format!("task-{}-todo-{}", task_id, todo)),

            // Base worktree (no todo_index)
            (None, Some(t), None) => Ok(format!("task/{}", t)),
            (None, None, None) => {
                let timestamp = Utc::now().timestamp();
                Ok(format!("task-{}-{}", task_id, timestamp))
            }
        }
    }

    fn determine_worktree_path(&self, repo_path: &str, branch: &str) -> Result<String> {
        let repo_path = Path::new(repo_path);
        let worktree_path = repo_path.join(branch);

        Ok(worktree_path.to_string_lossy().to_string())
    }

    fn create_git_worktree(
        &self,
        repo_path: &str,
        worktree_path: &str,
        branch: &str,
    ) -> Result<()> {
        let output = Command::new("git")
            .args([
                "-C",
                repo_path,
                "worktree",
                "add",
                "-b",
                branch,
                worktree_path,
            ])
            .output()?;

        if !output.status.success() {
            let error = String::from_utf8_lossy(&output.stderr);
            return Err(TrackError::Git(error.to_string()));
        }

        Ok(())
    }

    fn remove_git_worktree(&self, repo_path: &str, worktree_path: &str) -> Result<()> {
        let output = Command::new("git")
            .args(["-C", repo_path, "worktree", "remove", worktree_path])
            .output()?;

        if !output.status.success() {
            let error = String::from_utf8_lossy(&output.stderr);
            return Err(TrackError::Git(error.to_string()));
        }

        Ok(())
    }

    pub fn complete_worktree_for_todo(&self, todo_id: i64) -> Result<Option<String>> {
        let wt = match self.get_worktree_by_todo(todo_id)? {
            Some(wt) => wt,
            None => return Ok(None),
        };

        // Try to find a base worktree in the database
        let merge_target_path = if let Some(base_wt) = self.get_base_worktree(wt.task_id)? {
            // Use the registered base worktree
            base_wt.path
        } else {
            // Fall back to the base repository path (main repository directory)
            // The TODO worktree's base_repo field points to the main repository
            wt.base_repo.clone().ok_or_else(|| {
                TrackError::Other(
                    "TODO worktree has no base repository reference".to_string(),
                )
            })?
        };

        if self.has_uncommitted_changes(&wt.path)? {
            return Err(TrackError::Other(format!(
                "Worktree {} has uncommitted changes. Please commit or stash them.",
                wt.path
            )));
        }

        self.merge_branch(&merge_target_path, &wt.branch)?;
        self.remove_worktree(wt.id, false)?;

        Ok(Some(wt.branch))
    }

    fn get_worktree_by_todo(&self, todo_id: i64) -> Result<Option<GitItem>> {
        let conn = self.db.get_connection();
        let mut stmt = conn.prepare(
            "SELECT id, task_id, path, branch, base_repo, status, created_at, todo_id, is_base FROM git_items WHERE todo_id = ?1"
        )?;

        let result = stmt
            .query_row(params![todo_id], |row| {
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
            })
            .optional()?;

        Ok(result)
    }

    fn get_base_worktree(&self, task_id: i64) -> Result<Option<GitItem>> {
        let conn = self.db.get_connection();
        let mut stmt = conn.prepare(
            "SELECT id, task_id, path, branch, base_repo, status, created_at, todo_id, is_base FROM git_items WHERE task_id = ?1 AND is_base = 1"
        )?;

        let result = stmt
            .query_row(params![task_id], |row| {
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
            })
            .optional()?;

        Ok(result)
    }

    pub fn has_uncommitted_changes(&self, path: &str) -> Result<bool> {
        let output = Command::new("git")
            .args(["-C", path, "status", "--porcelain"])
            .output()?;

        Ok(!output.stdout.is_empty())
    }

    fn merge_branch(&self, target_path: &str, branch: &str) -> Result<()> {
        let output = Command::new("git")
            .args(["-C", target_path, "merge", "--no-ff", branch])
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
    fn test_determine_branch_name_with_explicit_branch_and_ticket() {
        let db = setup_db();
        let service = WorktreeService::new(&db);

        let result = service
            .determine_branch_name(Some("feature-x"), Some("PROJ-123"), 1, None)
            .unwrap();
        assert_eq!(result, "PROJ-123/feature-x");
    }

    #[test]
    fn test_determine_branch_name_with_explicit_branch_only() {
        let db = setup_db();
        let service = WorktreeService::new(&db);

        let result = service
            .determine_branch_name(Some("feature-y"), None, 1, None)
            .unwrap();
        assert_eq!(result, "feature-y");
    }

    #[test]
    fn test_determine_branch_name_with_ticket_and_todo() {
        let db = setup_db();
        let service = WorktreeService::new(&db);

        let result = service
            .determine_branch_name(None, Some("PROJ-456"), 1, Some(5))
            .unwrap();
        assert_eq!(result, "PROJ-456-todo-5");
    }

    #[test]
    fn test_determine_branch_name_with_todo_only() {
        let db = setup_db();
        let service = WorktreeService::new(&db);

        let result = service
            .determine_branch_name(None, None, 2, Some(7))
            .unwrap();
        assert_eq!(result, "task-2-todo-7");
    }

    #[test]
    fn test_determine_branch_name_base_with_ticket() {
        let db = setup_db();
        let service = WorktreeService::new(&db);

        let result = service
            .determine_branch_name(None, Some("PROJ-789"), 3, None)
            .unwrap();
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

    #[test]
    fn test_has_uncommitted_changes() {
        use std::fs::File;
        use std::process::Command;
        // Setup temp git repo
        let temp_dir = tempfile::tempdir().unwrap();
        let path = temp_dir.path().to_str().unwrap();

        Command::new("git").args(&["init", path]).output().unwrap();

        // Configure user for commit
        Command::new("git")
            .args(&["-C", path, "config", "user.email", "test@example.com"])
            .output()
            .unwrap();
        Command::new("git")
            .args(&["-C", path, "config", "user.name", "Test User"])
            .output()
            .unwrap();

        let db = setup_db();
        let service = WorktreeService::new(&db);

        // No changes initially
        assert!(!service.has_uncommitted_changes(path).unwrap());

        // Create a file (untracked)
        File::create(temp_dir.path().join("test.txt")).unwrap();
        // Untracked files count as changes with --porcelain
        assert!(service.has_uncommitted_changes(path).unwrap());

        // Commit it
        Command::new("git")
            .args(&["-C", path, "add", "."])
            .output()
            .unwrap();
        // Staged changes
        assert!(service.has_uncommitted_changes(path).unwrap());

        Command::new("git")
            .args(&["-C", path, "commit", "-m", "init"])
            .output()
            .unwrap();
        // Clean
        assert!(!service.has_uncommitted_changes(path).unwrap());

        // Modify
        std::fs::write(temp_dir.path().join("test.txt"), "mod").unwrap();
        assert!(service.has_uncommitted_changes(path).unwrap());
    }

    #[test]
    fn test_determine_worktree_path() {
        let db = setup_db();
        let service = WorktreeService::new(&db);

        // Test with simple path
        let repo_path = if cfg!(windows) {
            "C:\\path\\to\\repo"
        } else {
            "/path/to/repo"
        };
        let branch = "feature/test";

        let result = service.determine_worktree_path(repo_path, branch).unwrap();

        let expected = Path::new(repo_path)
            .join(branch)
            .to_string_lossy()
            .to_string();
        assert_eq!(result, expected);
    }

    #[test]
    fn test_add_worktree_and_get() {
        use crate::services::TaskService;
        use std::fs;
        use std::process::Command;

        let db = setup_db();
        let task_service = TaskService::new(&db);
        let service = WorktreeService::new(&db);

        // Create a task
        let task = task_service
            .create_task("Test Task", None, Some("PROJ-100"), None)
            .unwrap();

        // Create a temporary git repository
        let temp_dir = tempfile::tempdir().unwrap();
        let repo_path = temp_dir.path().to_str().unwrap();

        Command::new("git")
            .args(&["init", repo_path])
            .output()
            .unwrap();

        // Configure git user
        Command::new("git")
            .args(&["-C", repo_path, "config", "user.email", "test@example.com"])
            .output()
            .unwrap();
        Command::new("git")
            .args(&["-C", repo_path, "config", "user.name", "Test User"])
            .output()
            .unwrap();

        // Create initial commit
        fs::write(temp_dir.path().join("README.md"), "# Test").unwrap();
        Command::new("git")
            .args(&["-C", repo_path, "add", "."])
            .output()
            .unwrap();
        Command::new("git")
            .args(&["-C", repo_path, "commit", "-m", "Initial commit"])
            .output()
            .unwrap();

        // Add base worktree
        let worktree = service
            .add_worktree(task.id, repo_path, None, Some("PROJ-100"), None, true)
            .unwrap();

        assert_eq!(worktree.task_id, task.id);
        assert_eq!(worktree.branch, "task/PROJ-100");
        assert!(worktree.is_base);

        // Get worktree by ID
        let retrieved = service.get_git_item(worktree.id).unwrap();
        assert_eq!(retrieved.id, worktree.id);
        assert_eq!(retrieved.branch, "task/PROJ-100");
    }

    #[test]
    fn test_list_worktrees() {
        use crate::services::{TaskService, TodoService};
        use std::fs;
        use std::process::Command;

        let db = setup_db();
        let task_service = TaskService::new(&db);
        let todo_service = TodoService::new(&db);
        let service = WorktreeService::new(&db);

        // Create a task
        let task = task_service
            .create_task("Test Task", None, Some("PROJ-200"), None)
            .unwrap();

        // Create a TODO
        let todo = todo_service.add_todo(task.id, "Test TODO", true).unwrap();

        // Create a temporary git repository
        let temp_dir = tempfile::tempdir().unwrap();
        let repo_path = temp_dir.path().to_str().unwrap();

        Command::new("git")
            .args(&["init", repo_path])
            .output()
            .unwrap();

        Command::new("git")
            .args(&["-C", repo_path, "config", "user.email", "test@example.com"])
            .output()
            .unwrap();
        Command::new("git")
            .args(&["-C", repo_path, "config", "user.name", "Test User"])
            .output()
            .unwrap();

        fs::write(temp_dir.path().join("README.md"), "# Test").unwrap();
        Command::new("git")
            .args(&["-C", repo_path, "add", "."])
            .output()
            .unwrap();
        Command::new("git")
            .args(&["-C", repo_path, "commit", "-m", "Initial commit"])
            .output()
            .unwrap();

        // Add base worktree
        service
            .add_worktree(task.id, repo_path, None, Some("PROJ-200"), None, true)
            .unwrap();

        // Add TODO worktree
        service
            .add_worktree(
                task.id,
                repo_path,
                None,
                Some("PROJ-200"),
                Some(todo.id),
                false,
            )
            .unwrap();

        // List worktrees
        let worktrees = service.list_worktrees(task.id).unwrap();
        assert_eq!(worktrees.len(), 2);

        // Verify base worktree
        let base = worktrees.iter().find(|w| w.is_base).unwrap();
        assert_eq!(base.branch, "task/PROJ-200");

        // Verify TODO worktree
        let todo_wt = worktrees.iter().find(|w| !w.is_base).unwrap();
        assert_eq!(todo_wt.branch, "PROJ-200-todo-1");
        assert_eq!(todo_wt.todo_id, Some(todo.id));
    }

    #[test]
    fn test_remove_worktree() {
        use crate::services::TaskService;
        use std::fs;
        use std::process::Command;

        let db = setup_db();
        let task_service = TaskService::new(&db);
        let service = WorktreeService::new(&db);

        let task = task_service
            .create_task("Test Task", None, Some("PROJ-300"), None)
            .unwrap();

        let temp_dir = tempfile::tempdir().unwrap();
        let repo_path = temp_dir.path().to_str().unwrap();

        Command::new("git")
            .args(&["init", repo_path])
            .output()
            .unwrap();

        Command::new("git")
            .args(&["-C", repo_path, "config", "user.email", "test@example.com"])
            .output()
            .unwrap();
        Command::new("git")
            .args(&["-C", repo_path, "config", "user.name", "Test User"])
            .output()
            .unwrap();

        fs::write(temp_dir.path().join("README.md"), "# Test").unwrap();
        Command::new("git")
            .args(&["-C", repo_path, "add", "."])
            .output()
            .unwrap();
        Command::new("git")
            .args(&["-C", repo_path, "commit", "-m", "Initial commit"])
            .output()
            .unwrap();

        let worktree = service
            .add_worktree(task.id, repo_path, None, Some("PROJ-300"), None, true)
            .unwrap();

        // Verify worktree exists
        let worktrees_before = service.list_worktrees(task.id).unwrap();
        assert_eq!(worktrees_before.len(), 1);

        // Remove worktree
        service.remove_worktree(worktree.id, false).unwrap();

        // Verify worktree is removed from DB
        let worktrees_after = service.list_worktrees(task.id).unwrap();
        assert_eq!(worktrees_after.len(), 0);

        // Verify get_git_item returns error
        let result = service.get_git_item(worktree.id);
        assert!(result.is_err());
    }

    #[test]
    fn test_get_base_worktree() {
        use crate::services::{TaskService, TodoService};
        use std::fs;
        use std::process::Command;

        let db = setup_db();
        let task_service = TaskService::new(&db);
        let todo_service = TodoService::new(&db);
        let service = WorktreeService::new(&db);

        let task = task_service
            .create_task("Test Task", None, Some("PROJ-400"), None)
            .unwrap();

        let todo = todo_service.add_todo(task.id, "Test TODO", true).unwrap();

        let temp_dir = tempfile::tempdir().unwrap();
        let repo_path = temp_dir.path().to_str().unwrap();

        Command::new("git")
            .args(&["init", repo_path])
            .output()
            .unwrap();

        Command::new("git")
            .args(&["-C", repo_path, "config", "user.email", "test@example.com"])
            .output()
            .unwrap();
        Command::new("git")
            .args(&["-C", repo_path, "config", "user.name", "Test User"])
            .output()
            .unwrap();

        fs::write(temp_dir.path().join("README.md"), "# Test").unwrap();
        Command::new("git")
            .args(&["-C", repo_path, "add", "."])
            .output()
            .unwrap();
        Command::new("git")
            .args(&["-C", repo_path, "commit", "-m", "Initial commit"])
            .output()
            .unwrap();

        // Add base worktree
        let base_wt = service
            .add_worktree(task.id, repo_path, None, Some("PROJ-400"), None, true)
            .unwrap();

        // Add TODO worktree
        service
            .add_worktree(
                task.id,
                repo_path,
                None,
                Some("PROJ-400"),
                Some(todo.id),
                false,
            )
            .unwrap();

        // Get base worktree
        let retrieved_base = service.get_base_worktree(task.id).unwrap();
        assert!(retrieved_base.is_some());
        let base = retrieved_base.unwrap();
        assert_eq!(base.id, base_wt.id);
        assert!(base.is_base);
    }

    #[test]
    fn test_get_worktree_by_todo() {
        use crate::services::{TaskService, TodoService};
        use std::fs;
        use std::process::Command;

        let db = setup_db();
        let task_service = TaskService::new(&db);
        let todo_service = TodoService::new(&db);
        let service = WorktreeService::new(&db);

        let task = task_service
            .create_task("Test Task", None, Some("PROJ-500"), None)
            .unwrap();

        let todo = todo_service.add_todo(task.id, "Test TODO", true).unwrap();

        let temp_dir = tempfile::tempdir().unwrap();
        let repo_path = temp_dir.path().to_str().unwrap();

        Command::new("git")
            .args(&["init", repo_path])
            .output()
            .unwrap();

        Command::new("git")
            .args(&["-C", repo_path, "config", "user.email", "test@example.com"])
            .output()
            .unwrap();
        Command::new("git")
            .args(&["-C", repo_path, "config", "user.name", "Test User"])
            .output()
            .unwrap();

        fs::write(temp_dir.path().join("README.md"), "# Test").unwrap();
        Command::new("git")
            .args(&["-C", repo_path, "add", "."])
            .output()
            .unwrap();
        Command::new("git")
            .args(&["-C", repo_path, "commit", "-m", "Initial commit"])
            .output()
            .unwrap();

        // Add TODO worktree
        let todo_wt = service
            .add_worktree(
                task.id,
                repo_path,
                None,
                Some("PROJ-500"),
                Some(todo.id),
                false,
            )
            .unwrap();

        // Get worktree by TODO
        let retrieved = service.get_worktree_by_todo(todo.id).unwrap();
        assert!(retrieved.is_some());
        let wt = retrieved.unwrap();
        assert_eq!(wt.id, todo_wt.id);
        assert_eq!(wt.todo_id, Some(todo.id));
    }

    #[test]
    fn test_add_worktree_with_invalid_todo_id() {
        use crate::utils::TrackError;
        let db = setup_db();
        let service = WorktreeService::new(&db);

        let temp_dir = tempfile::tempdir().unwrap();
        let repo_path = temp_dir.path().to_str().unwrap();
        std::process::Command::new("git")
            .args(["init", repo_path])
            .output()
            .unwrap();

        let result = service.add_worktree(
            1, // task_id
            repo_path,
            None,
            None,
            Some(999), // Invalid TODO ID
            false,
        );

        match result {
            Err(TrackError::TodoNotFound(id)) => assert_eq!(id, 999),
            _ => panic!("Expected TodoNotFound error, got {:?}", result),
        }
    }

    #[test]
    fn test_add_worktree_validation() {
        use crate::utils::TrackError;
        let db = setup_db();
        let service = WorktreeService::new(&db);

        // 1. Invalid Git Repo
        let temp_dir = tempfile::tempdir().unwrap();
        let path = temp_dir.path().to_str().unwrap();
        // Don't init git

        let result = service.add_worktree(1, path, Some("b"), None, None, false);
        assert!(matches!(result, Err(TrackError::NotGitRepository(_))));

        // 2. Branch Exists
        let temp_dir2 = tempfile::tempdir().unwrap();
        let repo_path = temp_dir2.path().to_str().unwrap();
        std::process::Command::new("git")
            .args(["init", repo_path])
            .output()
            .unwrap();
        std::process::Command::new("git")
            .args(["-C", repo_path, "config", "user.email", "test@example.com"])
            .output()
            .unwrap();
        std::process::Command::new("git")
            .args(["-C", repo_path, "config", "user.name", "Test"])
            .output()
            .unwrap();
        std::fs::write(std::path::Path::new(repo_path).join("README.md"), "init").unwrap();
        std::process::Command::new("git")
            .args(["-C", repo_path, "add", "."])
            .output()
            .unwrap();
        std::process::Command::new("git")
            .args(["-C", repo_path, "commit", "-m", "init"])
            .output()
            .unwrap();

        // Create branch manualy
        std::process::Command::new("git")
            .args(["-C", repo_path, "branch", "existing-branch"])
            .output()
            .unwrap();

        let result = service.add_worktree(1, repo_path, Some("existing-branch"), None, None, false);
        // Should detect branch exists
        assert!(matches!(result, Err(TrackError::BranchExists(_))));
    }
}
