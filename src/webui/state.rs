//! Application state shared across handlers.

use crate::db::Database;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{broadcast, Mutex};

/// Event types broadcast via SSE
#[derive(Clone, Debug, serde::Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SseEvent {
    /// Description was updated
    Description,
    /// Ticket was updated
    Ticket,
    /// Links were updated
    Links,
    /// TODOs were updated
    Todos,
    /// Scraps were updated
    Scraps,
}

/// Database state snapshot for change detection
#[derive(Clone, Debug, PartialEq)]
struct DbSnapshot {
    task_count: i64,
    todo_count: i64,
    scrap_count: i64,
    link_count: i64,
    task_modified: Option<String>,
}

/// Shared application state
#[derive(Clone)]
pub struct AppState {
    /// Database connection wrapped for async access
    pub db: Arc<Mutex<Database>>,
    /// Broadcast channel for SSE events
    pub sse_tx: broadcast::Sender<SseEvent>,
    /// Last known database state for change detection
    last_snapshot: Arc<Mutex<Option<DbSnapshot>>>,
}

impl AppState {
    /// Create new application state with database connection
    pub fn new() -> anyhow::Result<Self> {
        let db = Database::new()?;
        let (sse_tx, _) = broadcast::channel(100);

        Ok(Self {
            db: Arc::new(Mutex::new(db)),
            sse_tx,
            last_snapshot: Arc::new(Mutex::new(None)),
        })
    }

    /// Broadcast an SSE event to all connected clients
    pub fn broadcast(&self, event: SseEvent) {
        // Ignore send errors (no receivers connected)
        let _ = self.sse_tx.send(event);
    }

    /// Get current database snapshot
    async fn get_snapshot(&self) -> anyhow::Result<DbSnapshot> {
        let db = self.db.lock().await;
        let conn = db.get_connection();

        let task_count: i64 = conn.query_row("SELECT COUNT(*) FROM tasks", [], |row| row.get(0))?;

        let todo_count: i64 = conn.query_row("SELECT COUNT(*) FROM todos", [], |row| row.get(0))?;

        let scrap_count: i64 =
            conn.query_row("SELECT COUNT(*) FROM scraps", [], |row| row.get(0))?;

        let link_count: i64 = conn.query_row("SELECT COUNT(*) FROM links", [], |row| row.get(0))?;

        // Get current task's last modification info
        let task_modified: Option<String> = if let Ok(Some(task_id)) = db.get_current_task_id() {
            conn.query_row(
                "SELECT ticket_id || ':' || COALESCE(description, '') FROM tasks WHERE id = ?1",
                [task_id],
                |row| row.get(0),
            )
            .ok()
        } else {
            None
        };

        Ok(DbSnapshot {
            task_count,
            todo_count,
            scrap_count,
            link_count,
            task_modified,
        })
    }

    /// Start background task to detect database changes
    pub async fn start_change_detection(&self) {
        let mut interval = tokio::time::interval(Duration::from_secs(1));

        loop {
            interval.tick().await;

            // Get current snapshot
            let current = match self.get_snapshot().await {
                Ok(snapshot) => snapshot,
                Err(e) => {
                    eprintln!("Error getting database snapshot: {}", e);
                    continue;
                }
            };

            // Compare with last snapshot and broadcast specific events
            let mut last = self.last_snapshot.lock().await;

            if let Some(ref prev) = *last {
                // Check each field and broadcast specific events
                if current.task_modified != prev.task_modified {
                    // Task metadata changed (description or ticket)
                    self.broadcast(SseEvent::Description);
                    self.broadcast(SseEvent::Ticket);
                }

                if current.link_count != prev.link_count {
                    self.broadcast(SseEvent::Links);
                }

                if current.todo_count != prev.todo_count {
                    self.broadcast(SseEvent::Todos);
                }

                if current.scrap_count != prev.scrap_count {
                    self.broadcast(SseEvent::Scraps);
                }
            }

            // Update last snapshot
            *last = Some(current);
        }
    }
}
