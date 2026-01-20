use crate::db::Database;
use crate::models::TaskRepo;
use crate::utils::{Result, TrackError};
use chrono::Utc;
use rusqlite::{params, OptionalExtension};
use std::path::{Path, PathBuf};

pub struct RepoService<'a> {
    db: &'a Database,
}

impl<'a> RepoService<'a> {
    pub fn new(db: &'a Database) -> Self {
        RepoService { db }
    }

    /// Register a repository to a task
    pub fn add_repo(
        &self,
        task_id: i64,
        repo_path: &str,
        base_branch: Option<String>,
        base_commit_hash: Option<String>,
    ) -> Result<TaskRepo> {
        // Resolve to absolute path
        let abs_path = self.resolve_absolute_path(repo_path)?;

        // Validate it's a JJ repository
        if !self.is_jj_repository(&abs_path)? {
            return Err(TrackError::Other(format!(
                "{} is not a JJ repository",
                abs_path.display()
            )));
        }

        let path_str = abs_path.to_string_lossy().to_string();
        let created_at = Utc::now().to_rfc3339();

        // Use transaction to make duplicate check + SELECT MAX + INSERT atomic
        self.db.with_transaction(|| {
            // Check if already registered
            let existing: Option<i64> = self
                .db
                .get_connection()
                .query_row(
                    "SELECT id FROM task_repos WHERE task_id = ?1 AND repo_path = ?2",
                    params![task_id, path_str],
                    |row| row.get(0),
                )
                .optional()?;

            if existing.is_some() {
                return Err(TrackError::Other(
                    "Repository already registered for this task".to_string(),
                ));
            }

            // Get the next task_index for this task
            let next_index: i64 = self.db.get_connection().query_row(
                "SELECT COALESCE(MAX(task_index), 0) + 1 FROM task_repos WHERE task_id = ?1",
                params![task_id],
                |row| row.get(0),
            )?;

            self.db.get_connection().execute(
                "INSERT INTO task_repos (task_id, task_index, repo_path, base_branch, base_commit_hash, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![task_id, next_index, path_str, base_branch, base_commit_hash, created_at],
            )?;

            let id = self.db.get_connection().last_insert_rowid();

            self.db.increment_rev("repos")?;
            Ok(TaskRepo {
                id,
                task_id,
                task_index: next_index,
                repo_path: path_str.clone(),
                base_branch,
                base_commit_hash,
                created_at: Utc::now(),
            })
        })
    }

    /// List all repositories for a task
    pub fn list_repos(&self, task_id: i64) -> Result<Vec<TaskRepo>> {
        let mut stmt = self.db.get_connection().prepare(
            "SELECT id, task_id, task_index, repo_path, base_branch, base_commit_hash, created_at FROM task_repos WHERE task_id = ?1 ORDER BY task_index"
        )?;

        let repos = stmt
            .query_map(params![task_id], |row| {
                Ok(TaskRepo {
                    id: row.get(0)?,
                    task_id: row.get(1)?,
                    task_index: row.get(2)?,
                    repo_path: row.get(3)?,
                    base_branch: row.get(4)?,
                    base_commit_hash: row.get(5)?,
                    created_at: row
                        .get::<_, String>(6)?
                        .parse()
                        .unwrap_or_else(|_| Utc::now()),
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        Ok(repos)
    }

    /// Remove a repository registration
    pub fn remove_repo(&self, repo_id: i64) -> Result<()> {
        let rows_affected = self
            .db
            .get_connection()
            .execute("DELETE FROM task_repos WHERE id = ?1", params![repo_id])?;

        if rows_affected == 0 {
            return Err(TrackError::Other(format!(
                "Repository #{} not found",
                repo_id
            )));
        }

        self.db.increment_rev("repos")?;
        Ok(())
    }

    /// Resolve path to absolute path
    fn resolve_absolute_path(&self, path: &str) -> Result<PathBuf> {
        let path_buf = PathBuf::from(path);

        if path_buf.is_absolute() {
            Ok(path_buf)
        } else {
            std::env::current_dir()?
                .join(path_buf)
                .canonicalize()
                .map_err(|e| TrackError::Other(format!("Failed to resolve path: {}", e)))
        }
    }

    /// Check if a path is a JJ repository
    fn is_jj_repository(&self, path: &Path) -> Result<bool> {
        let jj_dir = path.join(".jj");
        Ok(jj_dir.exists())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::task_service::TaskService;

    fn setup_db() -> Database {
        Database::new_in_memory().unwrap()
    }

    #[test]
    fn test_add_repo_success() {
        let db = setup_db();
        let task_service = TaskService::new(&db);
        let repo_service = RepoService::new(&db);

        // Create a task
        let task = task_service
            .create_task("Test Task", None, None, None)
            .unwrap();

        // Create a temporary JJ repository
        let temp_dir = std::env::temp_dir().join(format!("test_repo_{}", std::process::id()));
        std::fs::create_dir_all(&temp_dir).unwrap();
        std::fs::create_dir(temp_dir.join(".jj")).unwrap();

        // Add the repository
        let repo = repo_service
            .add_repo(task.id, temp_dir.to_str().unwrap(), None, None)
            .unwrap();
        assert_eq!(repo.task_id, task.id);
        assert!(repo.repo_path.contains("test_repo"));

        // Cleanup
        std::fs::remove_dir_all(&temp_dir).ok();
    }

    #[test]
    fn test_add_repo_not_git() {
        let db = setup_db();
        let task_service = TaskService::new(&db);
        let repo_service = RepoService::new(&db);

        let task = task_service
            .create_task("Test Task", None, None, None)
            .unwrap();

        // Create a temporary directory without .jj
        let temp_dir = std::env::temp_dir().join(format!("test_not_git_{}", std::process::id()));
        std::fs::create_dir_all(&temp_dir).unwrap();

        // Try to add the repository
        let result = repo_service.add_repo(task.id, temp_dir.to_str().unwrap(), None, None);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("not a JJ repository"));

        // Cleanup
        std::fs::remove_dir_all(&temp_dir).ok();
    }

    #[test]
    fn test_add_repo_duplicate() {
        let db = setup_db();
        let task_service = TaskService::new(&db);
        let repo_service = RepoService::new(&db);

        let task = task_service
            .create_task("Test Task", None, None, None)
            .unwrap();

        let temp_dir = std::env::temp_dir().join(format!("test_dup_repo_{}", std::process::id()));
        std::fs::create_dir_all(&temp_dir).unwrap();
        std::fs::create_dir(temp_dir.join(".jj")).unwrap();

        // Add the repository twice
        repo_service
            .add_repo(task.id, temp_dir.to_str().unwrap(), None, None)
            .unwrap();
        let result = repo_service.add_repo(task.id, temp_dir.to_str().unwrap(), None, None);

        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("already registered"));

        // Cleanup
        std::fs::remove_dir_all(&temp_dir).ok();
    }

    #[test]
    fn test_list_repos() {
        let db = setup_db();
        let task_service = TaskService::new(&db);
        let repo_service = RepoService::new(&db);

        let task = task_service
            .create_task("Test Task", None, None, None)
            .unwrap();

        // Create two temporary JJ repositories
        let temp_dir1 =
            std::env::temp_dir().join(format!("test_list_repo1_{}", std::process::id()));
        let temp_dir2 =
            std::env::temp_dir().join(format!("test_list_repo2_{}", std::process::id()));
        std::fs::create_dir_all(&temp_dir1).unwrap();
        std::fs::create_dir_all(&temp_dir2).unwrap();
        std::fs::create_dir(temp_dir1.join(".jj")).unwrap();
        std::fs::create_dir(temp_dir2.join(".jj")).unwrap();

        // Add both repositories
        repo_service
            .add_repo(task.id, temp_dir1.to_str().unwrap(), None, None)
            .unwrap();
        repo_service
            .add_repo(task.id, temp_dir2.to_str().unwrap(), None, None)
            .unwrap();

        // List repositories
        let repos = repo_service.list_repos(task.id).unwrap();
        assert_eq!(repos.len(), 2);

        // Cleanup
        std::fs::remove_dir_all(&temp_dir1).ok();
        std::fs::remove_dir_all(&temp_dir2).ok();
    }

    #[test]
    fn test_remove_repo() {
        let db = setup_db();
        let task_service = TaskService::new(&db);
        let repo_service = RepoService::new(&db);

        let task = task_service
            .create_task("Test Task", None, None, None)
            .unwrap();

        let temp_dir =
            std::env::temp_dir().join(format!("test_remove_repo_{}", std::process::id()));
        std::fs::create_dir_all(&temp_dir).unwrap();
        std::fs::create_dir(temp_dir.join(".jj")).unwrap();

        let repo = repo_service
            .add_repo(task.id, temp_dir.to_str().unwrap(), None, None)
            .unwrap();

        // Remove the repository
        repo_service.remove_repo(repo.id).unwrap();

        // Verify it's removed
        let repos = repo_service.list_repos(task.id).unwrap();
        assert_eq!(repos.len(), 0);
    }

    #[test]
    fn test_add_repo_with_base_info() {
        let db = setup_db();
        let task_service = TaskService::new(&db);
        let repo_service = RepoService::new(&db);

        let task = task_service
            .create_task("Test Task", None, None, None)
            .unwrap();

        let temp_dir = std::env::temp_dir().join(format!("test_repo_base_{}", std::process::id()));
        std::fs::create_dir_all(&temp_dir).unwrap();
        std::fs::create_dir(temp_dir.join(".jj")).unwrap();

        // Add repository with base branch and commit hash
        let base_branch = "main".to_string();
        let base_hash = "abc123def456".to_string();

        let repo = repo_service
            .add_repo(
                task.id,
                temp_dir.to_str().unwrap(),
                Some(base_branch.clone()),
                Some(base_hash.clone()),
            )
            .unwrap();

        assert_eq!(repo.base_branch, Some(base_branch.clone()));
        assert_eq!(repo.base_commit_hash, Some(base_hash.clone()));

        // Verify data persists by listing repos
        let repos = repo_service.list_repos(task.id).unwrap();
        assert_eq!(repos.len(), 1);
        assert_eq!(repos[0].base_branch, Some(base_branch));
        assert_eq!(repos[0].base_commit_hash, Some(base_hash));

        // Cleanup
        std::fs::remove_dir_all(&temp_dir).ok();
    }
}
