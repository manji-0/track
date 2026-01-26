use crate::db::Database;
use crate::models::{RepoLink, Worktree};
use crate::utils::{Result, TrackError};
use chrono::{DateTime, Utc};
use rusqlite::{params, types::Type, OptionalExtension};
use std::path::Path;
use std::process::Command;

fn parse_datetime(value: String) -> rusqlite::Result<DateTime<Utc>> {
    value
        .parse()
        .map_err(|e| rusqlite::Error::FromSqlConversionFailure(0, Type::Text, Box::new(e)))
}

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
    ) -> Result<Worktree> {
        // Verify it's a JJ repository
        if !self.is_jj_repository(repo_path)? {
            return Err(TrackError::NotJjRepository(repo_path.to_string()));
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

        // Determine bookmark name
        let branch_name = self.determine_branch_name(branch, ticket_id, task_id, todo_index)?;

        // Check if bookmark already exists
        if self.bookmark_exists(repo_path, &branch_name)? {
            return Err(TrackError::BookmarkExists(branch_name));
        }

        // Determine workspace path
        let worktree_path = self.determine_worktree_path(repo_path, &branch_name)?;

        let task_bookmark = self.task_bookmark_name(task_id, ticket_id);
        let base_revset = if is_base { "@" } else { task_bookmark.as_str() };

        // Create workspace and bookmark
        self.create_jj_workspace(repo_path, &worktree_path, &branch_name, base_revset)?;

        let worktree = self.insert_worktree_record(
            task_id,
            &worktree_path,
            &branch_name,
            repo_path,
            todo_id,
            is_base,
        )?;

        Ok(worktree)
    }

    fn insert_worktree_record(
        &self,
        task_id: i64,
        worktree_path: &str,
        branch_name: &str,
        repo_path: &str,
        todo_id: Option<i64>,
        is_base: bool,
    ) -> Result<Worktree> {
        let now = Utc::now().to_rfc3339();
        let conn = self.db.get_connection();

        conn.execute(
            "INSERT INTO worktrees (task_id, path, branch, base_repo, status, created_at, todo_id, is_base) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![task_id, worktree_path, branch_name, repo_path, "active", now, todo_id, is_base as i32],
        )?;

        let worktree_id = conn.last_insert_rowid();
        self.db.increment_rev("worktrees")?;
        self.get_worktree(worktree_id)
    }

    pub fn add_existing_worktree(
        &self,
        task_id: i64,
        repo_path: &str,
        branch: &str,
        todo_id: Option<i64>,
        is_base: bool,
        worktree_path: Option<&str>,
    ) -> Result<Worktree> {
        if !self.is_jj_repository(repo_path)? {
            return Err(TrackError::NotJjRepository(repo_path.to_string()));
        }

        let resolved_path = if let Some(path) = worktree_path {
            path.to_string()
        } else {
            self.determine_worktree_path(repo_path, branch)?
        };

        self.create_jj_workspace_for_existing_bookmark(repo_path, &resolved_path, branch)?;

        self.insert_worktree_record(task_id, &resolved_path, branch, repo_path, todo_id, is_base)
    }

    pub fn recreate_worktree(&self, worktree: &Worktree, force: bool) -> Result<Worktree> {
        let repo_path = worktree.base_repo.as_ref().ok_or_else(|| {
            TrackError::Other("Worktree has no base repository reference".to_string())
        })?;

        if Path::new(&worktree.path).exists() {
            if self.has_uncommitted_changes(&worktree.path)? && !force {
                return Err(TrackError::Other(format!(
                    "Worktree {} has uncommitted changes. Use --force to recreate.",
                    worktree.path
                )));
            }

            self.remove_jj_workspace(repo_path, &worktree.path)?;
        }

        let conn = self.db.get_connection();
        conn.execute("DELETE FROM worktrees WHERE id = ?1", params![worktree.id])?;
        self.db.increment_rev("worktrees")?;

        self.add_existing_worktree(
            worktree.task_id,
            repo_path,
            &worktree.branch,
            worktree.todo_id,
            worktree.is_base,
            Some(&worktree.path),
        )
    }

    pub fn get_worktree(&self, worktree_id: i64) -> Result<Worktree> {
        let conn = self.db.get_connection();
        let mut stmt = conn.prepare(
            "SELECT id, task_id, path, branch, base_repo, status, created_at, todo_id, is_base FROM worktrees WHERE id = ?1"
        )?;

        let worktree = stmt
            .query_row(params![worktree_id], |row| {
                let is_base: i32 = row.get(8).unwrap_or(0);
                Ok(Worktree {
                    id: row.get(0)?,
                    task_id: row.get(1)?,
                    path: row.get(2)?,
                    branch: row.get(3)?,
                    base_repo: row.get(4)?,
                    status: row.get(5)?,
                    created_at: parse_datetime(row.get::<_, String>(6)?)?,
                    todo_id: row.get(7)?,
                    is_base: is_base != 0,
                })
            })
            .map_err(|_| TrackError::WorktreeNotFound(worktree_id))?;

        Ok(worktree)
    }

    pub fn list_worktrees(&self, task_id: i64) -> Result<Vec<Worktree>> {
        let conn = self.db.get_connection();
        let mut stmt = conn.prepare(
            "SELECT id, task_id, path, branch, base_repo, status, created_at, todo_id, is_base FROM worktrees WHERE task_id = ?1 ORDER BY created_at ASC"
        )?;

        let worktrees = stmt
            .query_map(params![task_id], |row| {
                let is_base: i32 = row.get(8).unwrap_or(0);
                Ok(Worktree {
                    id: row.get(0)?,
                    task_id: row.get(1)?,
                    path: row.get(2)?,
                    branch: row.get(3)?,
                    base_repo: row.get(4)?,
                    status: row.get(5)?,
                    created_at: parse_datetime(row.get::<_, String>(6)?)?,
                    todo_id: row.get(7)?,
                    is_base: is_base != 0,
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        Ok(worktrees)
    }

    pub fn list_repo_links(&self, worktree_id: i64) -> Result<Vec<RepoLink>> {
        let conn = self.db.get_connection();
        let mut stmt = conn.prepare(
            "SELECT id, worktree_id, url, kind, created_at FROM repo_links WHERE worktree_id = ?1 ORDER BY created_at ASC"
        )?;

        let repo_links = stmt
            .query_map(params![worktree_id], |row| {
                Ok(RepoLink {
                    id: row.get(0)?,
                    worktree_id: row.get(1)?,
                    url: row.get(2)?,
                    kind: row.get(3)?,
                    created_at: parse_datetime(row.get::<_, String>(4)?)?,
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        Ok(repo_links)
    }

    pub fn remove_worktree(&self, worktree_id: i64, keep_files: bool) -> Result<()> {
        let worktree = self.get_worktree(worktree_id)?;

        if !keep_files {
            // Remove JJ workspace
            if let Some(base_repo) = &worktree.base_repo {
                self.remove_jj_workspace(base_repo, &worktree.path)?;
            }
        }

        // Remove from database
        let conn = self.db.get_connection();
        conn.execute("DELETE FROM worktrees WHERE id = ?1", params![worktree_id])?;

        self.db.increment_rev("worktrees")?;
        Ok(())
    }

    fn is_jj_repository(&self, path: &str) -> Result<bool> {
        let jj_dir = Path::new(path).join(".jj");
        Ok(jj_dir.exists())
    }

    pub fn bookmark_exists_in_repo(&self, repo_path: &str, bookmark: &str) -> Result<bool> {
        self.bookmark_exists(repo_path, bookmark)
    }

    fn bookmark_exists(&self, repo_path: &str, bookmark: &str) -> Result<bool> {
        if !self.is_jj_repository(repo_path)? {
            return Err(TrackError::NotJjRepository(repo_path.to_string()));
        }

        let output = Command::new("jj")
            .current_dir(repo_path)
            .args(["-R", repo_path, "bookmark", "list", bookmark])
            .output()?;

        if !output.status.success() {
            return Ok(false);
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        Ok(stdout
            .lines()
            .any(|line| line.trim_start().starts_with(&format!("{}:", bookmark))))
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

    fn task_bookmark_name(&self, task_id: i64, ticket_id: Option<&str>) -> String {
        if let Some(ticket) = ticket_id {
            format!("task/{}", ticket)
        } else {
            format!("task/task-{}", task_id)
        }
    }

    fn determine_worktree_path(&self, repo_path: &str, branch: &str) -> Result<String> {
        let sanitized_branch = branch.replace(['/', '\\'], "_");
        let repo_path = Path::new(repo_path);
        let worktree_path = repo_path.join(sanitized_branch);

        Ok(worktree_path.to_string_lossy().to_string())
    }

    fn create_jj_workspace(
        &self,
        repo_path: &str,
        worktree_path: &str,
        bookmark: &str,
        base_revset: &str,
    ) -> Result<()> {
        let output = Command::new("jj")
            .current_dir(repo_path)
            .args([
                "-R",
                repo_path,
                "workspace",
                "add",
                worktree_path,
                "-r",
                base_revset,
            ])
            .output()?;

        if !output.status.success() {
            let error = String::from_utf8_lossy(&output.stderr);
            return Err(TrackError::Jj(error.to_string()));
        }

        let bookmark_output = Command::new("jj")
            .current_dir(worktree_path)
            .args([
                "-R",
                worktree_path,
                "bookmark",
                "create",
                bookmark,
                "-r",
                "@",
            ])
            .output()?;

        if !bookmark_output.status.success() {
            let error = String::from_utf8_lossy(&bookmark_output.stderr);
            return Err(TrackError::Jj(error.to_string()));
        }

        Ok(())
    }

    fn create_jj_workspace_for_existing_bookmark(
        &self,
        repo_path: &str,
        worktree_path: &str,
        bookmark: &str,
    ) -> Result<()> {
        let output = Command::new("jj")
            .current_dir(repo_path)
            .args(["-R", repo_path, "workspace", "add", worktree_path])
            .output()?;

        if !output.status.success() {
            let error = String::from_utf8_lossy(&output.stderr);
            return Err(TrackError::Jj(error.to_string()));
        }

        let edit_output = Command::new("jj")
            .current_dir(worktree_path)
            .args(["-R", worktree_path, "edit", bookmark])
            .output()?;

        if !edit_output.status.success() {
            let error = String::from_utf8_lossy(&edit_output.stderr);
            return Err(TrackError::Jj(error.to_string()));
        }

        Ok(())
    }

    fn remove_jj_workspace(&self, repo_path: &str, worktree_path: &str) -> Result<()> {
        let workspace_name = Path::new(worktree_path)
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or_else(|| TrackError::Other("Failed to determine workspace name".to_string()))?;

        let output = Command::new("jj")
            .current_dir(repo_path)
            .args(["-R", repo_path, "workspace", "forget", workspace_name])
            .output()?;

        if !output.status.success() {
            let error = String::from_utf8_lossy(&output.stderr);
            return Err(TrackError::Jj(error.to_string()));
        }

        if Path::new(worktree_path).exists() {
            std::fs::remove_dir_all(worktree_path)?;
        }

        Ok(())
    }

    /// Calculates the expected bookmark name for a TODO item.
    /// This allows clients to know the bookmark name even before the workspace is created.
    pub fn get_todo_branch_name(
        &self,
        task_id: i64,
        ticket_id: Option<&str>,
        todo_index: i64,
    ) -> Result<String> {
        self.determine_branch_name(None, ticket_id, task_id, Some(todo_index))
    }

    fn get_task_ticket_id(&self, task_id: i64) -> Result<Option<String>> {
        let conn = self.db.get_connection();
        let mut stmt = conn.prepare("SELECT ticket_id FROM tasks WHERE id = ?1")?;
        let ticket_id = stmt
            .query_row(params![task_id], |row| row.get::<_, Option<String>>(0))
            .optional()?;
        Ok(ticket_id.flatten())
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
                TrackError::Other("TODO workspace has no base repository reference".to_string())
            })?
        };

        if self.has_uncommitted_changes(&wt.path)? {
            return Err(TrackError::Other(format!(
                "Workspace {} has uncommitted changes. Please clean or snapshot them.",
                wt.path
            )));
        }

        let ticket_id = self.get_task_ticket_id(wt.task_id)?;
        let task_bookmark = self.task_bookmark_name(wt.task_id, ticket_id.as_deref());

        self.integrate_todo_bookmark(&merge_target_path, &wt.branch, &task_bookmark)?;
        self.remove_worktree(wt.id, false)?;

        Ok(Some(wt.branch))
    }

    fn get_worktree_by_todo(&self, todo_id: i64) -> Result<Option<Worktree>> {
        let conn = self.db.get_connection();
        let mut stmt = conn.prepare(
            "SELECT id, task_id, path, branch, base_repo, status, created_at, todo_id, is_base FROM worktrees WHERE todo_id = ?1"
        )?;

        let result = stmt
            .query_row(params![todo_id], |row| {
                let is_base: i32 = row.get(8).unwrap_or(0);
                Ok(Worktree {
                    id: row.get(0)?,
                    task_id: row.get(1)?,
                    path: row.get(2)?,
                    branch: row.get(3)?,
                    base_repo: row.get(4)?,
                    status: row.get(5)?,
                    created_at: parse_datetime(row.get::<_, String>(6)?)?,
                    todo_id: row.get(7)?,
                    is_base: is_base != 0,
                })
            })
            .optional()?;

        Ok(result)
    }

    fn get_base_worktree(&self, task_id: i64) -> Result<Option<Worktree>> {
        let conn = self.db.get_connection();
        let mut stmt = conn.prepare(
            "SELECT id, task_id, path, branch, base_repo, status, created_at, todo_id, is_base FROM worktrees WHERE task_id = ?1 AND is_base = 1"
        )?;

        let result = stmt
            .query_row(params![task_id], |row| {
                let is_base: i32 = row.get(8).unwrap_or(0);
                Ok(Worktree {
                    id: row.get(0)?,
                    task_id: row.get(1)?,
                    path: row.get(2)?,
                    branch: row.get(3)?,
                    base_repo: row.get(4)?,
                    status: row.get(5)?,
                    created_at: parse_datetime(row.get::<_, String>(6)?)?,
                    todo_id: row.get(7)?,
                    is_base: is_base != 0,
                })
            })
            .optional()?;

        Ok(result)
    }

    pub fn has_uncommitted_changes(&self, path: &str) -> Result<bool> {
        let output = Command::new("jj")
            .current_dir(path)
            .args(["-R", path, "diff", "--summary"])
            .output()?;

        Ok(!output.stdout.is_empty())
    }

    fn integrate_todo_bookmark(
        &self,
        target_path: &str,
        todo_bookmark: &str,
        task_bookmark: &str,
    ) -> Result<()> {
        let rebase_output = Command::new("jj")
            .current_dir(target_path)
            .args([
                "-R",
                target_path,
                "rebase",
                "-r",
                todo_bookmark,
                "-d",
                task_bookmark,
            ])
            .output()?;

        if !rebase_output.status.success() {
            let error = String::from_utf8_lossy(&rebase_output.stderr);
            return Err(TrackError::Jj(format!("Rebase failed: {}", error)));
        }

        let move_output = Command::new("jj")
            .current_dir(target_path)
            .args([
                "-R",
                target_path,
                "bookmark",
                "move",
                task_bookmark,
                "-t",
                todo_bookmark,
            ])
            .output()?;

        if !move_output.status.success() {
            let error = String::from_utf8_lossy(&move_output.stderr);
            return Err(TrackError::Jj(format!("Bookmark move failed: {}", error)));
        }

        let edit_output = Command::new("jj")
            .current_dir(target_path)
            .args(["-R", target_path, "edit", task_bookmark])
            .output()?;

        if !edit_output.status.success() {
            let error = String::from_utf8_lossy(&edit_output.stderr);
            return Err(TrackError::Jj(format!(
                "Workspace update failed: {}",
                error
            )));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Database;
    use std::process::Command;

    fn setup_db() -> Database {
        Database::new_in_memory().unwrap()
    }

    fn init_jj_repo(path: &str) {
        Command::new("jj")
            .args(["git", "init", path])
            .output()
            .unwrap();
    }

    fn describe_change(path: &str, message: &str) {
        Command::new("jj")
            .args(["-R", path, "describe", "-m", message])
            .output()
            .unwrap();
    }

    fn new_change(path: &str) {
        Command::new("jj")
            .args(["-R", path, "new"])
            .output()
            .unwrap();
    }

    fn create_bookmark(path: &str, name: &str) {
        Command::new("jj")
            .args(["-R", path, "bookmark", "create", name, "-r", "@"])
            .output()
            .unwrap();
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
        // Setup temp JJ repo
        let temp_dir = tempfile::tempdir().unwrap();
        let path = temp_dir.path().to_str().unwrap();

        init_jj_repo(path);

        let db = setup_db();
        let service = WorktreeService::new(&db);

        // No changes initially
        assert!(!service.has_uncommitted_changes(path).unwrap());

        // Create a file (untracked)
        File::create(temp_dir.path().join("test.txt")).unwrap();
        assert!(service.has_uncommitted_changes(path).unwrap());

        // Record the change and move to a clean working copy
        describe_change(path, "init");
        new_change(path);
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
            .join("feature_test")
            .to_string_lossy()
            .to_string();
        assert_eq!(result, expected);
    }

    #[test]
    fn test_add_worktree_and_get() {
        use crate::services::TaskService;
        use std::fs;

        let db = setup_db();
        let task_service = TaskService::new(&db);
        let service = WorktreeService::new(&db);

        // Create a task
        let task = task_service
            .create_task("Test Task", None, Some("PROJ-100"), None)
            .unwrap();

        // Create a temporary JJ repository
        let temp_dir = tempfile::tempdir().unwrap();
        let repo_path = temp_dir.path().to_str().unwrap();

        init_jj_repo(repo_path);

        // Create initial change
        fs::write(temp_dir.path().join("README.md"), "# Test").unwrap();
        describe_change(repo_path, "Initial commit");

        // Add base worktree
        let worktree = service
            .add_worktree(task.id, repo_path, None, Some("PROJ-100"), None, true)
            .unwrap();

        assert_eq!(worktree.task_id, task.id);
        assert_eq!(worktree.branch, "task/PROJ-100");
        assert!(worktree.is_base);

        // Get worktree by ID
        let retrieved = service.get_worktree(worktree.id).unwrap();
        assert_eq!(retrieved.id, worktree.id);
        assert_eq!(retrieved.branch, "task/PROJ-100");
    }

    #[test]
    fn test_list_worktrees() {
        use crate::services::{TaskService, TodoService};
        use std::fs;

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

        // Create a temporary JJ repository
        let temp_dir = tempfile::tempdir().unwrap();
        let repo_path = temp_dir.path().to_str().unwrap();

        init_jj_repo(repo_path);
        fs::write(temp_dir.path().join("README.md"), "# Test").unwrap();
        describe_change(repo_path, "Initial commit");

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

        let db = setup_db();
        let task_service = TaskService::new(&db);
        let service = WorktreeService::new(&db);

        let task = task_service
            .create_task("Test Task", None, Some("PROJ-300"), None)
            .unwrap();

        let temp_dir = tempfile::tempdir().unwrap();
        let repo_path = temp_dir.path().to_str().unwrap();

        init_jj_repo(repo_path);
        fs::write(temp_dir.path().join("README.md"), "# Test").unwrap();
        describe_change(repo_path, "Initial commit");

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

        // Verify get_worktree returns error
        let result = service.get_worktree(worktree.id);
        assert!(result.is_err());
    }

    #[test]
    fn test_get_base_worktree() {
        use crate::services::{TaskService, TodoService};
        use std::fs;

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

        init_jj_repo(repo_path);
        fs::write(temp_dir.path().join("README.md"), "# Test").unwrap();
        describe_change(repo_path, "Initial commit");

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

        init_jj_repo(repo_path);
        fs::write(temp_dir.path().join("README.md"), "# Test").unwrap();
        describe_change(repo_path, "Initial commit");
        create_bookmark(repo_path, "task/PROJ-500");

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
    fn test_get_worktree_by_todo_not_found() {
        use crate::services::{TaskService, TodoService};

        let db = setup_db();
        let task_service = TaskService::new(&db);
        let todo_service = TodoService::new(&db);
        let service = WorktreeService::new(&db);

        let task = task_service
            .create_task("Test Task", None, Some("PROJ-501"), None)
            .unwrap();

        let todo = todo_service.add_todo(task.id, "Test TODO", true).unwrap();

        // Don't create any worktree for this TODO
        // Try to get worktree by TODO
        let retrieved = service.get_worktree_by_todo(todo.id).unwrap();
        assert!(retrieved.is_none());
    }

    #[test]
    fn test_get_worktree_by_todo_is_base_field() {
        use crate::services::{TaskService, TodoService};
        use std::fs;

        let db = setup_db();
        let task_service = TaskService::new(&db);
        let todo_service = TodoService::new(&db);
        let service = WorktreeService::new(&db);

        let task = task_service
            .create_task("Test Task", None, Some("PROJ-502"), None)
            .unwrap();

        let todo = todo_service.add_todo(task.id, "Test TODO", true).unwrap();

        let temp_dir = tempfile::tempdir().unwrap();
        let repo_path = temp_dir.path().to_str().unwrap();

        init_jj_repo(repo_path);
        fs::write(temp_dir.path().join("README.md"), "# Test").unwrap();
        describe_change(repo_path, "Initial commit");
        create_bookmark(repo_path, "task/PROJ-502");

        // Add TODO worktree (not base)
        service
            .add_worktree(
                task.id,
                repo_path,
                None,
                Some("PROJ-502"),
                Some(todo.id),
                false, // is_base = false
            )
            .unwrap();

        // Get worktree by TODO and verify is_base is false
        let retrieved = service.get_worktree_by_todo(todo.id).unwrap();
        assert!(retrieved.is_some());
        let wt = retrieved.unwrap();
        assert_eq!(wt.is_base, false);
    }

    #[test]
    fn test_add_worktree_with_invalid_todo_id() {
        use crate::utils::TrackError;
        let db = setup_db();
        let service = WorktreeService::new(&db);

        let temp_dir = tempfile::tempdir().unwrap();
        let repo_path = temp_dir.path().to_str().unwrap();
        init_jj_repo(repo_path);

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

        // 1. Invalid JJ Repo
        let temp_dir = tempfile::tempdir().unwrap();
        let path = temp_dir.path().to_str().unwrap();
        // Don't init jj

        let result = service.add_worktree(1, path, Some("b"), None, None, false);
        assert!(matches!(result, Err(TrackError::NotJjRepository(_))));

        // 2. Bookmark Exists
        let temp_dir2 = tempfile::tempdir().unwrap();
        let repo_path = temp_dir2.path().to_str().unwrap();
        init_jj_repo(repo_path);
        std::fs::write(std::path::Path::new(repo_path).join("README.md"), "init").unwrap();
        describe_change(repo_path, "init");
        create_bookmark(repo_path, "existing-branch");

        let result = service.add_worktree(1, repo_path, Some("existing-branch"), None, None, false);
        // Should detect bookmark exists
        assert!(matches!(result, Err(TrackError::BookmarkExists(_))));
    }
}
