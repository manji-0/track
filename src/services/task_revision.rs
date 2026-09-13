//! Task-level VCS revision stored when aggressive mode is on.

use crate::db::Database;
use crate::db::row_mapping::parse_datetime;
use crate::models::TaskId;
use crate::utils::Result;
use chrono::{DateTime, Utc};
use rusqlite::{OptionalExtension, params};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskRevision {
    pub task_id: TaskId,
    pub repo_path: String,
    pub git_commit: String,
    pub jj_change_id: Option<String>,
    pub created_at: DateTime<Utc>,
}

pub struct TaskRevisionService<'a> {
    db: &'a Database,
}

impl<'a> TaskRevisionService<'a> {
    pub fn new(db: &'a Database) -> Self {
        Self { db }
    }

    pub fn get(&self, task_id: TaskId, repo_path: &str) -> Result<Option<TaskRevision>> {
        let conn = self.db.get_connection();
        let mut stmt = conn.prepare(
            "SELECT task_id, repo_path, git_commit, jj_change_id, created_at
             FROM task_revisions WHERE task_id = ?1 AND repo_path = ?2",
        )?;
        stmt.query_row(params![task_id, repo_path], map_row)
            .optional()
            .map_err(Into::into)
    }

    pub fn list_for_task(&self, task_id: TaskId) -> Result<Vec<TaskRevision>> {
        let conn = self.db.get_connection();
        let mut stmt = conn.prepare(
            "SELECT task_id, repo_path, git_commit, jj_change_id, created_at
             FROM task_revisions WHERE task_id = ?1 ORDER BY created_at ASC",
        )?;
        let rows = stmt
            .query_map(params![task_id], map_row)?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    /// Records the marker revision. Existing rows are left unchanged so notes
    /// stay attached to the original commit.
    pub fn insert_if_absent(
        &self,
        task_id: TaskId,
        repo_path: &str,
        git_commit: &str,
        jj_change_id: Option<&str>,
    ) -> Result<TaskRevision> {
        if let Some(existing) = self.get(task_id, repo_path)? {
            return Ok(existing);
        }
        let now = Utc::now().to_rfc3339();
        let conn = self.db.get_connection();
        conn.execute(
            "INSERT INTO task_revisions (task_id, repo_path, git_commit, jj_change_id, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5)
             ON CONFLICT(task_id, repo_path) DO NOTHING",
            params![task_id, repo_path, git_commit, jj_change_id, now],
        )?;
        self.get(task_id, repo_path)?
            .ok_or_else(|| crate::utils::TrackError::PathResolutionFailed(repo_path.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Database;
    use crate::services::TaskService;

    #[test]
    fn insert_if_absent_keeps_original_commit() {
        let db = Database::new_in_memory().unwrap();
        let task = TaskService::new(&db)
            .create_task("Rev", None, None, None)
            .unwrap();
        let svc = TaskRevisionService::new(&db);
        let first = svc
            .insert_if_absent(task.id, "/repo", "aaa", Some("change-a"))
            .unwrap();
        let second = svc
            .insert_if_absent(task.id, "/repo", "bbb", Some("change-b"))
            .unwrap();
        assert_eq!(first.git_commit, "aaa");
        assert_eq!(second.git_commit, "aaa");
        assert_eq!(second.jj_change_id.as_deref(), Some("change-a"));
    }
}

fn map_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<TaskRevision> {
    Ok(TaskRevision {
        task_id: row.get(0)?,
        repo_path: row.get(1)?,
        git_commit: row.get(2)?,
        jj_change_id: row.get(3)?,
        created_at: parse_datetime(row.get::<_, String>(4)?)?,
    })
}
