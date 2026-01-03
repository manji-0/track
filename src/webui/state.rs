//! Application state shared across handlers.

use crate::db::Database;
use std::sync::Arc;
use tokio::sync::{broadcast, Mutex};

/// Event types broadcast via SSE
#[derive(Clone, Debug, serde::Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SseEvent {
    /// Full status update needed
    StatusUpdate,
    /// A new TODO was added
    TodoAdded { todo_id: i64 },
    /// A TODO was deleted
    TodoDeleted { todo_id: i64 },
    /// A TODO status changed
    #[allow(dead_code)]
    TodoUpdated { todo_id: i64 },
    /// A new scrap was added
    ScrapAdded { id: i64 },
}

/// Shared application state
#[derive(Clone)]
pub struct AppState {
    /// Database connection wrapped for async access
    pub db: Arc<Mutex<Database>>,
    /// Broadcast channel for SSE events
    pub sse_tx: broadcast::Sender<SseEvent>,
}

impl AppState {
    /// Create new application state with database connection
    pub fn new() -> anyhow::Result<Self> {
        let db = Database::new()?;
        let (sse_tx, _) = broadcast::channel(100);

        Ok(Self {
            db: Arc::new(Mutex::new(db)),
            sse_tx,
        })
    }

    /// Broadcast an SSE event to all connected clients
    pub fn broadcast(&self, event: SseEvent) {
        // Ignore send errors (no receivers connected)
        let _ = self.sse_tx.send(event);
    }
}
