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
    pub fn add_repo(&self, task_id: i64, repo_path: &str) -> Result<TaskRepo> {
        // Resolve to absolute path
        let abs_path = self.resolve_absolute_path(repo_path)?;

        // Validate it's a Git repository
        if !self.is_git_repository(&abs_path)? {
            return Err(TrackError::Other(format!(
                "{} is not a Git repository",
                abs_path.display()
            )));
        }

        let path_str = abs_path.to_string_lossy().to_string();

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

        // Insert the repository
        let created_at = Utc::now().to_rfc3339();
        self.db.get_connection().execute(
            "INSERT INTO task_repos (task_id, repo_path, created_at) VALUES (?1, ?2, ?3)",
            params![task_id, path_str, created_at],
        )?;

        let id = self.db.get_connection().last_insert_rowid();

        Ok(TaskRepo {
            id,
            task_id,
            repo_path: path_str,
            created_at: Utc::now(),
        })
    }

    /// List all repositories for a task
    pub fn list_repos(&self, task_id: i64) -> Result<Vec<TaskRepo>> {
        let mut stmt = self.db.get_connection().prepare(
            "SELECT id, task_id, repo_path, created_at FROM task_repos WHERE task_id = ?1 ORDER BY created_at"
        )?;

        let repos = stmt
            .query_map(params![task_id], |row| {
                Ok(TaskRepo {
                    id: row.get(0)?,
                    task_id: row.get(1)?,
                    repo_path: row.get(2)?,
                    created_at: row
                        .get::<_, String>(3)?
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

    /// Check if a path is a Git repository
    fn is_git_repository(&self, path: &Path) -> Result<bool> {
        let git_dir = path.join(".git");
        Ok(git_dir.exists())
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

        // Create a temporary git repository
        let temp_dir = std::env::temp_dir().join(format!("test_repo_{}", std::process::id()));
        std::fs::create_dir_all(&temp_dir).unwrap();
        std::fs::create_dir(temp_dir.join(".git")).unwrap();

        // Add the repository
        let repo = repo_service
            .add_repo(task.id, temp_dir.to_str().unwrap())
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

        // Create a temporary directory without .git
        let temp_dir = std::env::temp_dir().join(format!("test_not_git_{}", std::process::id()));
        std::fs::create_dir_all(&temp_dir).unwrap();

        // Try to add the repository
        let result = repo_service.add_repo(task.id, temp_dir.to_str().unwrap());
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("not a Git repository"));

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
        std::fs::create_dir(temp_dir.join(".git")).unwrap();

        // Add the repository twice
        repo_service
            .add_repo(task.id, temp_dir.to_str().unwrap())
            .unwrap();
        let result = repo_service.add_repo(task.id, temp_dir.to_str().unwrap());

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

        // Create two temporary git repositories
        let temp_dir1 =
            std::env::temp_dir().join(format!("test_list_repo1_{}", std::process::id()));
        let temp_dir2 =
            std::env::temp_dir().join(format!("test_list_repo2_{}", std::process::id()));
        std::fs::create_dir_all(&temp_dir1).unwrap();
        std::fs::create_dir_all(&temp_dir2).unwrap();
        std::fs::create_dir(temp_dir1.join(".git")).unwrap();
        std::fs::create_dir(temp_dir2.join(".git")).unwrap();

        // Add both repositories
        repo_service
            .add_repo(task.id, temp_dir1.to_str().unwrap())
            .unwrap();
        repo_service
            .add_repo(task.id, temp_dir2.to_str().unwrap())
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
        std::fs::create_dir(temp_dir.join(".git")).unwrap();

        let repo = repo_service
            .add_repo(task.id, temp_dir.to_str().unwrap())
            .unwrap();

        // Remove the repository
        repo_service.remove_repo(repo.id).unwrap();

        // Verify it's removed
        let repos = repo_service.list_repos(task.id).unwrap();
        assert_eq!(repos.len(), 0);

        // Cleanup
        std::fs::remove_dir_all(&temp_dir).ok();
    }
}
