mod jj;
mod naming;

use crate::db::row_mapping::parse_datetime;
use crate::db::Database;
use crate::models::{RepoLink, Worktree};
use crate::utils::{Result, TrackError};
use chrono::Utc;
use rusqlite::{params, OptionalExtension};
use std::path::Path;

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
        if !jj::is_jj_repository(repo_path) {
            return Err(TrackError::NotJjRepository(repo_path.to_string()));
        }

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

        let branch_name = naming::determine_branch_name(branch, ticket_id, task_id, todo_index)?;

        if jj::bookmark_exists(repo_path, &branch_name)? {
            return Err(TrackError::BookmarkExists(branch_name));
        }

        let worktree_path = naming::determine_worktree_path(repo_path, &branch_name)?;
        let task_bookmark = naming::task_bookmark_name(task_id, ticket_id);
        let base_revset = if is_base { "@" } else { task_bookmark.as_str() };

        jj::create_workspace(repo_path, &worktree_path, &branch_name, base_revset)?;

        self.insert_worktree_record(
            task_id,
            &worktree_path,
            &branch_name,
            repo_path,
            todo_id,
            is_base,
        )
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
        if !jj::is_jj_repository(repo_path) {
            return Err(TrackError::NotJjRepository(repo_path.to_string()));
        }

        let resolved_path = if let Some(path) = worktree_path {
            path.to_string()
        } else {
            naming::determine_worktree_path(repo_path, branch)?
        };

        jj::create_workspace_for_existing_bookmark(repo_path, &resolved_path, branch)?;

        self.insert_worktree_record(task_id, &resolved_path, branch, repo_path, todo_id, is_base)
    }

    pub fn recreate_worktree(&self, worktree: &Worktree, force: bool) -> Result<Worktree> {
        let repo_path = worktree.base_repo.as_ref().ok_or_else(|| {
            TrackError::PathResolutionFailed(format!(
                "worktree #{} has no base repository reference",
                worktree.id
            ))
        })?;

        if Path::new(&worktree.path).exists() {
            if jj::has_uncommitted_changes(&worktree.path)? && !force {
                return Err(TrackError::WorkspaceHasUncommittedChanges {
                    path: worktree.path.clone(),
                });
            }

            jj::remove_workspace(repo_path, &worktree.path)?;
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

        stmt.query_row(params![worktree_id], map_worktree_row)
            .map_err(|_| TrackError::WorktreeNotFound(worktree_id))
    }

    pub fn list_worktrees(&self, task_id: i64) -> Result<Vec<Worktree>> {
        let conn = self.db.get_connection();
        let mut stmt = conn.prepare(
            "SELECT id, task_id, path, branch, base_repo, status, created_at, todo_id, is_base FROM worktrees WHERE task_id = ?1 ORDER BY created_at ASC"
        )?;

        let worktrees = stmt
            .query_map(params![task_id], map_worktree_row)?
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
            if let Some(base_repo) = &worktree.base_repo {
                jj::remove_workspace(base_repo, &worktree.path)?;
            }
        }

        let conn = self.db.get_connection();
        conn.execute("DELETE FROM worktrees WHERE id = ?1", params![worktree_id])?;
        self.db.increment_rev("worktrees")?;
        Ok(())
    }

    pub fn bookmark_exists_in_repo(&self, repo_path: &str, bookmark: &str) -> Result<bool> {
        jj::bookmark_exists(repo_path, bookmark)
    }

    pub fn task_bookmark_name(&self, task_id: i64, ticket_id: Option<&str>) -> String {
        naming::task_bookmark_name(task_id, ticket_id)
    }

    pub fn get_todo_branch_name(
        &self,
        task_id: i64,
        ticket_id: Option<&str>,
        todo_index: i64,
    ) -> Result<String> {
        naming::determine_branch_name(None, ticket_id, task_id, Some(todo_index))
    }

    pub fn complete_worktree_for_todo(&self, todo_id: i64) -> Result<Option<String>> {
        let wt = match self.get_worktree_by_todo(todo_id)? {
            Some(wt) => wt,
            None => return Ok(None),
        };

        let merge_target_path = if let Some(base_wt) = self.get_base_worktree(wt.task_id)? {
            base_wt.path
        } else {
            wt.base_repo
                .clone()
                .ok_or(TrackError::NoWorkspacePathsAvailable)?
        };

        if jj::has_uncommitted_changes(&wt.path)? {
            return Err(TrackError::WorkspaceHasUncommittedChanges {
                path: wt.path.clone(),
            });
        }

        let ticket_id = self.get_task_ticket_id(wt.task_id)?;
        let task_bookmark = naming::task_bookmark_name(wt.task_id, ticket_id.as_deref());

        jj::integrate_todo_bookmark(&merge_target_path, &wt.branch, &task_bookmark)?;
        self.remove_worktree(wt.id, false)?;

        Ok(Some(wt.branch))
    }

    pub fn has_uncommitted_changes(&self, path: &str) -> Result<bool> {
        jj::has_uncommitted_changes(path)
    }

    fn get_task_ticket_id(&self, task_id: i64) -> Result<Option<String>> {
        let conn = self.db.get_connection();
        let mut stmt = conn.prepare("SELECT ticket_id FROM tasks WHERE id = ?1")?;
        let ticket_id = stmt
            .query_row(params![task_id], |row| row.get::<_, Option<String>>(0))
            .optional()?;
        Ok(ticket_id.flatten())
    }

    fn get_worktree_by_todo(&self, todo_id: i64) -> Result<Option<Worktree>> {
        let conn = self.db.get_connection();
        let mut stmt = conn.prepare(
            "SELECT id, task_id, path, branch, base_repo, status, created_at, todo_id, is_base FROM worktrees WHERE todo_id = ?1"
        )?;

        stmt.query_row(params![todo_id], map_worktree_row)
            .optional()
            .map_err(TrackError::from)
    }

    fn get_base_worktree(&self, task_id: i64) -> Result<Option<Worktree>> {
        let conn = self.db.get_connection();
        let mut stmt = conn.prepare(
            "SELECT id, task_id, path, branch, base_repo, status, created_at, todo_id, is_base FROM worktrees WHERE task_id = ?1 AND is_base = 1"
        )?;

        stmt.query_row(params![task_id], map_worktree_row)
            .optional()
            .map_err(TrackError::from)
    }
}

fn map_worktree_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<Worktree> {
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::TaskService;
    use std::fs;
    use std::process::Command;

    fn setup_db() -> Database {
        Database::new_in_memory().unwrap()
    }

    fn jj_available() -> bool {
        Command::new("jj")
            .arg("--version")
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false)
    }

    fn require_jj() -> bool {
        if !jj_available() {
            eprintln!("Skipping test: jj binary not available");
            return false;
        }
        true
    }

    fn init_jj_repo(path: &str) {
        let output = Command::new("jj")
            .args(["git", "init", path])
            .output()
            .expect("failed to run jj git init");
        assert!(
            output.status.success(),
            "jj git init failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    fn describe_change(path: &str, message: &str) {
        let output = Command::new("jj")
            .args(["-R", path, "describe", "-m", message])
            .output()
            .expect("failed to run jj describe");
        assert!(
            output.status.success(),
            "jj describe failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    fn create_bookmark(path: &str, name: &str) {
        let output = Command::new("jj")
            .args(["-R", path, "bookmark", "create", name, "-r", "@"])
            .output()
            .expect("failed to run jj bookmark create");
        assert!(
            output.status.success(),
            "jj bookmark create failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    #[test]
    fn test_add_worktree_and_get() {
        if !require_jj() {
            return;
        }

        let db = setup_db();
        let task_service = TaskService::new(&db);
        let service = WorktreeService::new(&db);

        let task = task_service
            .create_task("Test Task", None, Some("PROJ-100"), None)
            .unwrap();

        let temp_dir = tempfile::tempdir().unwrap();
        let repo_path = temp_dir.path().to_str().unwrap();
        init_jj_repo(repo_path);
        fs::write(temp_dir.path().join("README.md"), "# Test").unwrap();
        describe_change(repo_path, "Initial commit");

        let worktree = service
            .add_worktree(task.id, repo_path, None, Some("PROJ-100"), None, true)
            .unwrap();

        assert_eq!(worktree.task_id, task.id);
        assert_eq!(worktree.branch, "task/PROJ-100");
        assert!(worktree.is_base);

        let retrieved = service.get_worktree(worktree.id).unwrap();
        assert_eq!(retrieved.id, worktree.id);
        assert_eq!(retrieved.branch, "task/PROJ-100");
    }

    #[test]
    fn test_list_worktrees() {
        use crate::services::TodoService;
        if !require_jj() {
            return;
        }

        let db = setup_db();
        let task_service = TaskService::new(&db);
        let todo_service = TodoService::new(&db);
        let service = WorktreeService::new(&db);

        let task = task_service
            .create_task("Test Task", None, Some("PROJ-200"), None)
            .unwrap();
        let todo = todo_service.add_todo(task.id, "Test TODO", true).unwrap();

        let temp_dir = tempfile::tempdir().unwrap();
        let repo_path = temp_dir.path().to_str().unwrap();
        init_jj_repo(repo_path);
        fs::write(temp_dir.path().join("README.md"), "# Test").unwrap();
        describe_change(repo_path, "Initial commit");

        service
            .add_worktree(task.id, repo_path, None, Some("PROJ-200"), None, true)
            .unwrap();
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

        let worktrees = service.list_worktrees(task.id).unwrap();
        assert_eq!(worktrees.len(), 2);
        assert!(worktrees.iter().any(|wt| wt.is_base));
        assert!(worktrees.iter().any(|wt| !wt.is_base));
    }

    #[test]
    fn test_remove_worktree() {
        use crate::services::TodoService;
        if !require_jj() {
            return;
        }

        let db = setup_db();
        let task_service = TaskService::new(&db);
        let todo_service = TodoService::new(&db);
        let service = WorktreeService::new(&db);

        let task = task_service
            .create_task("Test Task", None, Some("PROJ-300"), None)
            .unwrap();
        let todo = todo_service.add_todo(task.id, "Test TODO", true).unwrap();

        let temp_dir = tempfile::tempdir().unwrap();
        let repo_path = temp_dir.path().to_str().unwrap();
        init_jj_repo(repo_path);
        fs::write(temp_dir.path().join("README.md"), "# Test").unwrap();
        describe_change(repo_path, "Initial commit");

        service
            .add_worktree(task.id, repo_path, None, Some("PROJ-300"), None, true)
            .unwrap();
        let todo_wt = service
            .add_worktree(
                task.id,
                repo_path,
                None,
                Some("PROJ-300"),
                Some(todo.id),
                false,
            )
            .unwrap();

        assert!(Path::new(&todo_wt.path).exists());
        service.remove_worktree(todo_wt.id, false).unwrap();
        assert!(!Path::new(&todo_wt.path).exists());
        assert_eq!(service.list_worktrees(task.id).unwrap().len(), 1);
    }

    #[test]
    fn test_get_base_worktree() {
        if !require_jj() {
            return;
        }

        let db = setup_db();
        let task_service = TaskService::new(&db);
        let service = WorktreeService::new(&db);

        let task = task_service
            .create_task("Test Task", None, Some("PROJ-400"), None)
            .unwrap();

        let temp_dir = tempfile::tempdir().unwrap();
        let repo_path = temp_dir.path().to_str().unwrap();
        init_jj_repo(repo_path);
        fs::write(temp_dir.path().join("README.md"), "# Test").unwrap();
        describe_change(repo_path, "Initial commit");

        service
            .add_worktree(task.id, repo_path, None, Some("PROJ-400"), None, true)
            .unwrap();

        let base = service.get_base_worktree(task.id).unwrap();
        assert!(base.is_some());
        assert!(base.unwrap().is_base);
    }

    #[test]
    fn test_get_worktree_by_todo() {
        use crate::services::TodoService;
        if !require_jj() {
            return;
        }

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

        service
            .add_worktree(task.id, repo_path, None, Some("PROJ-500"), None, true)
            .unwrap();
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

        let found = service.get_worktree_by_todo(todo.id).unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().id, todo_wt.id);
    }

    #[test]
    fn test_get_worktree_by_todo_not_found() {
        let db = setup_db();
        let service = WorktreeService::new(&db);
        assert!(service.get_worktree_by_todo(999).unwrap().is_none());
    }

    #[test]
    fn test_get_worktree_by_todo_is_base_field() {
        use crate::services::TodoService;
        if !require_jj() {
            return;
        }

        let db = setup_db();
        let task_service = TaskService::new(&db);
        let todo_service = TodoService::new(&db);
        let service = WorktreeService::new(&db);

        let task = task_service
            .create_task("Test Task", None, Some("PROJ-600"), None)
            .unwrap();
        let todo = todo_service.add_todo(task.id, "Test TODO", true).unwrap();

        let temp_dir = tempfile::tempdir().unwrap();
        let repo_path = temp_dir.path().to_str().unwrap();
        init_jj_repo(repo_path);
        fs::write(temp_dir.path().join("README.md"), "# Test").unwrap();
        describe_change(repo_path, "Initial commit");

        service
            .add_worktree(task.id, repo_path, None, Some("PROJ-600"), None, true)
            .unwrap();
        service
            .add_worktree(
                task.id,
                repo_path,
                None,
                Some("PROJ-600"),
                Some(todo.id),
                false,
            )
            .unwrap();

        let todo_wt = service.get_worktree_by_todo(todo.id).unwrap().unwrap();
        assert!(!todo_wt.is_base);
    }

    #[test]
    fn test_add_worktree_with_invalid_todo_id() {
        if !require_jj() {
            return;
        }

        let db = setup_db();
        let service = WorktreeService::new(&db);
        let temp_dir = tempfile::tempdir().unwrap();
        let repo_path = temp_dir.path().to_str().unwrap();
        init_jj_repo(repo_path);

        let result = service.add_worktree(1, repo_path, None, Some("PROJ-999"), Some(999), false);
        assert!(matches!(result, Err(TrackError::TodoNotFound(999))));
    }

    #[test]
    fn test_add_worktree_validation() {
        let db = setup_db();
        let service = WorktreeService::new(&db);

        let path = if cfg!(windows) {
            "C:\\nonexistent\\path"
        } else {
            "/nonexistent/path"
        };
        let result = service.add_worktree(1, path, Some("b"), None, None, false);
        assert!(matches!(result, Err(TrackError::NotJjRepository(_))));

        if !require_jj() {
            return;
        }

        let temp_dir = tempfile::tempdir().unwrap();
        let repo_path = temp_dir.path().to_str().unwrap();
        init_jj_repo(repo_path);
        fs::write(temp_dir.path().join("README.md"), "# Test").unwrap();
        describe_change(repo_path, "Initial commit");
        create_bookmark(repo_path, "existing-branch");

        let result = service.add_worktree(1, repo_path, Some("existing-branch"), None, None, false);
        assert!(matches!(result, Err(TrackError::BookmarkExists(_))));
    }
}
