//! Application state shared across handlers.

use crate::db::{Database, SectionRevs};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{broadcast, Mutex};

/// Event types broadcast via SSE
#[derive(Clone, Debug, serde::Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SseEvent {
    /// Task header (name, alias) was updated
    Header,
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
    /// Worktrees were updated
    Worktrees,
    /// Repositories were updated
    Repos,
}

/// State snapshot for change detection using revision numbers
#[derive(Clone, Debug, PartialEq)]
struct ChangeState {
    current_task_id: Option<i64>,
    revs: SectionRevs,
}

/// Shared argument state
#[derive(Clone)]
pub struct AppState {
    /// Database connection wrapped for async access
    pub db: Arc<Mutex<Database>>,
    /// Broadcast channel for SSE events
    pub sse_tx: broadcast::Sender<SseEvent>,
    /// Last known state for change detection
    last_state: Arc<Mutex<Option<ChangeState>>>,
}

impl AppState {
    /// Create new application state with database connection
    pub fn new() -> anyhow::Result<Self> {
        let db = Database::new()?;
        let (sse_tx, _) = broadcast::channel(100);

        Ok(Self {
            db: Arc::new(Mutex::new(db)),
            sse_tx,
            last_state: Arc::new(Mutex::new(None)),
        })
    }

    /// Broadcast an SSE event to all connected clients
    pub fn broadcast(&self, event: SseEvent) {
        // Ignore send errors (no receivers connected)
        let _ = self.sse_tx.send(event);
    }

    /// Get current change state (task ID and all revision numbers)
    async fn get_change_state(&self) -> anyhow::Result<ChangeState> {
        let db = self.db.lock().await;
        let current_task_id = db.get_current_task_id()?;
        let revs = db.get_all_revs()?;

        Ok(ChangeState {
            current_task_id,
            revs,
        })
    }

    /// Broadcast all section events (used on task switch)
    fn broadcast_all(&self) {
        self.broadcast(SseEvent::Header);
        self.broadcast(SseEvent::Description);
        self.broadcast(SseEvent::Ticket);
        self.broadcast(SseEvent::Links);
        self.broadcast(SseEvent::Todos);
        self.broadcast(SseEvent::Scraps);
        self.broadcast(SseEvent::Repos);
        self.broadcast(SseEvent::Worktrees);
    }

    /// Start background task to detect database changes
    pub async fn start_change_detection(&self) {
        let mut interval = tokio::time::interval(Duration::from_secs(1));

        // Initialize with current state
        {
            let mut last = self.last_state.lock().await;
            if last.is_none() {
                if let Ok(initial_state) = self.get_change_state().await {
                    *last = Some(initial_state);
                }
            }
        }

        loop {
            interval.tick().await;

            // Get current state
            let current = match self.get_change_state().await {
                Ok(state) => state,
                Err(e) => {
                    eprintln!("Error getting change state: {}", e);
                    continue;
                }
            };

            // Compare with last state and broadcast specific events
            let mut last = self.last_state.lock().await;

            if let Some(ref prev) = *last {
                // Check if current task changed (task switch or new task)
                if current.current_task_id != prev.current_task_id {
                    // Task switched - reload all sections
                    self.broadcast_all();
                } else {
                    // Same task - check for specific rev changes
                    if current.revs.task != prev.revs.task {
                        // Task metadata changed (description, ticket, or alias)
                        self.broadcast(SseEvent::Header);
                        self.broadcast(SseEvent::Description);
                        self.broadcast(SseEvent::Ticket);
                    }

                    if current.revs.links != prev.revs.links {
                        self.broadcast(SseEvent::Links);
                    }

                    // TODOs are affected by both todos and worktrees revisions
                    if current.revs.todos != prev.revs.todos
                        || current.revs.worktrees != prev.revs.worktrees
                    {
                        self.broadcast(SseEvent::Todos);
                    }

                    if current.revs.repos != prev.revs.repos {
                        self.broadcast(SseEvent::Repos);
                    }

                    if current.revs.scraps != prev.revs.scraps {
                        self.broadcast(SseEvent::Scraps);
                    }
                }
            }

            // Update last state
            *last = Some(current);
        }
    }
}
