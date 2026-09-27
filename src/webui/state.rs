//! Application state shared across handlers.

use crate::db::Database;
use crate::models::TaskId;
use crate::utils::Result;
use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{Mutex, broadcast};

/// Event types broadcast via SSE
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
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
    /// The single-mode current task changed (`track switch` / `track new`)
    CurrentTask,
    /// A task was created or its metadata changed (task list pages)
    Tasks,
}

impl SseEvent {
    /// SSE `event:` name consumed by `hx-trigger="sse:..."`.
    pub fn event_name(&self) -> &'static str {
        match self {
            Self::Header => "header",
            Self::Description => "description",
            Self::Ticket => "ticket",
            Self::Links => "links",
            Self::Todos => "todos",
            Self::Scraps => "scraps",
            Self::Worktrees => "worktrees",
            Self::Repos => "repos",
            Self::CurrentTask => "current_task",
            Self::Tasks => "tasks",
        }
    }

    /// Section events fired when the `task_revs` counter for `section` moves.
    fn for_section(section: &str) -> &'static [SseEvent] {
        match section {
            "task" => &[Self::Header, Self::Description, Self::Ticket],
            "links" => &[Self::Links],
            // TODO rows show workspace paths, so worktree changes refresh them too.
            "todos" | "worktrees" => &[Self::Todos],
            "repos" => &[Self::Repos],
            "scraps" => &[Self::Scraps],
            _ => &[],
        }
    }
}

/// An SSE event plus the task it concerns (`None` for global events).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SseMessage {
    pub task_id: Option<TaskId>,
    pub event: SseEvent,
}

/// Which messages one SSE connection (one browser tab) receives.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SseSubscription {
    /// Task shown in the tab; `None` on task list / empty pages.
    pub task_id: Option<TaskId>,
    /// The tab follows the single-mode current task (`/`) and reloads when it changes.
    pub follow_current: bool,
}

impl SseSubscription {
    pub fn accepts(&self, message: &SseMessage) -> bool {
        match message.event {
            SseEvent::CurrentTask => self.follow_current,
            SseEvent::Tasks => self.task_id.is_none(),
            _ => message.task_id.is_some() && message.task_id == self.task_id,
        }
    }
}

/// State snapshot for change detection using per-task revision numbers
#[derive(Clone, Debug, PartialEq)]
struct ChangeState {
    current_task_id: Option<TaskId>,
    task_revs: BTreeMap<(TaskId, String), i64>,
}

impl ChangeState {
    /// Messages describing what changed between `prev` and `self`.
    fn diff(&self, prev: &ChangeState) -> Vec<SseMessage> {
        let mut messages = Vec::new();
        let mut push = |message: SseMessage| {
            if !messages.contains(&message) {
                messages.push(message);
            }
        };

        if self.current_task_id != prev.current_task_id {
            push(SseMessage {
                task_id: None,
                event: SseEvent::CurrentTask,
            });
        }

        for ((task_id, section), rev) in &self.task_revs {
            let key = (*task_id, section.clone());
            if prev.task_revs.get(&key) == Some(rev) {
                continue;
            }
            if section == "task" {
                push(SseMessage {
                    task_id: None,
                    event: SseEvent::Tasks,
                });
            }
            for event in SseEvent::for_section(section) {
                push(SseMessage {
                    task_id: Some(*task_id),
                    event: *event,
                });
            }
        }

        messages
    }
}

/// Shared argument state
#[derive(Clone)]
pub struct AppState {
    /// Database connection wrapped for async access
    pub db: Arc<Mutex<Database>>,
    /// Broadcast channel for SSE events
    pub sse_tx: broadcast::Sender<SseMessage>,
    /// Last known state for change detection
    last_state: Arc<Mutex<Option<ChangeState>>>,
}

impl AppState {
    /// Create application state backed by an existing database (for tests).
    pub fn from_database(db: Database) -> Self {
        let (sse_tx, _) = broadcast::channel(100);

        Self {
            db: Arc::new(Mutex::new(db)),
            sse_tx,
            last_state: Arc::new(Mutex::new(None)),
        }
    }

    /// Create new application state with database connection
    pub fn new() -> Result<Self> {
        Ok(Self::from_database(Database::new()?))
    }

    /// Broadcast a section event to the tabs showing `task_id`
    pub fn broadcast(&self, task_id: TaskId, event: SseEvent) {
        self.send(SseMessage {
            task_id: Some(task_id),
            event,
        });
    }

    fn send(&self, message: SseMessage) {
        // Ignore send errors (no receivers connected)
        let _ = self.sse_tx.send(message);
    }

    /// Get current change state (current task ID and per-task revision numbers)
    async fn get_change_state(&self) -> Result<ChangeState> {
        let db = self.db.lock().await;
        Ok(ChangeState {
            current_task_id: db.get_current_task_id()?,
            task_revs: db.get_task_revs()?,
        })
    }

    /// Start background task to detect database changes
    pub async fn start_change_detection(&self) {
        let mut interval = tokio::time::interval(Duration::from_secs(1));

        // Initialize with current state
        {
            let mut last = self.last_state.lock().await;
            if last.is_none()
                && let Ok(initial_state) = self.get_change_state().await
            {
                *last = Some(initial_state);
            }
        }

        loop {
            interval.tick().await;

            let current = match self.get_change_state().await {
                Ok(state) => state,
                Err(e) => {
                    eprintln!("Error getting change state: {}", e);
                    continue;
                }
            };

            let mut last = self.last_state.lock().await;
            if let Some(ref prev) = *last {
                for message in current.diff(prev) {
                    self.send(message);
                }
            }
            *last = Some(current);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn state(current: Option<i64>, revs: &[(i64, &str, i64)]) -> ChangeState {
        ChangeState {
            current_task_id: current.map(TaskId::from_i64),
            task_revs: revs
                .iter()
                .map(|(id, section, rev)| ((TaskId::from_i64(*id), section.to_string()), *rev))
                .collect(),
        }
    }

    #[test]
    fn sse_event_names_match_htmx_triggers() {
        assert_eq!(SseEvent::Header.event_name(), "header");
        assert_eq!(SseEvent::Description.event_name(), "description");
        assert_eq!(SseEvent::Ticket.event_name(), "ticket");
        assert_eq!(SseEvent::Links.event_name(), "links");
        assert_eq!(SseEvent::Todos.event_name(), "todos");
        assert_eq!(SseEvent::Scraps.event_name(), "scraps");
        assert_eq!(SseEvent::Worktrees.event_name(), "worktrees");
        assert_eq!(SseEvent::Repos.event_name(), "repos");
        assert_eq!(SseEvent::CurrentTask.event_name(), "current_task");
        assert_eq!(SseEvent::Tasks.event_name(), "tasks");
    }

    #[test]
    fn diff_scopes_section_events_to_the_changed_task() {
        let prev = state(Some(1), &[(1, "todos", 1), (2, "todos", 1)]);
        let next = state(Some(1), &[(1, "todos", 1), (2, "todos", 2)]);

        assert_eq!(
            next.diff(&prev),
            vec![SseMessage {
                task_id: Some(TaskId::from_i64(2)),
                event: SseEvent::Todos,
            }]
        );
    }

    #[test]
    fn diff_reports_current_task_switch_and_new_tasks_globally() {
        let prev = state(Some(1), &[(1, "task", 1)]);
        let next = state(Some(2), &[(1, "task", 1), (2, "task", 1)]);
        let messages = next.diff(&prev);

        assert!(messages.contains(&SseMessage {
            task_id: None,
            event: SseEvent::CurrentTask,
        }));
        assert!(messages.contains(&SseMessage {
            task_id: None,
            event: SseEvent::Tasks,
        }));
        assert!(messages.contains(&SseMessage {
            task_id: Some(TaskId::from_i64(2)),
            event: SseEvent::Header,
        }));
    }

    #[test]
    fn subscription_filters_by_tab_scope() {
        let task = |id| Some(TaskId::from_i64(id));
        let todos_for_1 = SseMessage {
            task_id: task(1),
            event: SseEvent::Todos,
        };
        let switched = SseMessage {
            task_id: None,
            event: SseEvent::CurrentTask,
        };
        let tasks = SseMessage {
            task_id: None,
            event: SseEvent::Tasks,
        };

        let pinned = SseSubscription {
            task_id: task(1),
            follow_current: false,
        };
        assert!(pinned.accepts(&todos_for_1));
        assert!(!pinned.accepts(&switched));
        assert!(!pinned.accepts(&tasks));

        let other = SseSubscription {
            task_id: task(2),
            follow_current: true,
        };
        assert!(!other.accepts(&todos_for_1));
        assert!(other.accepts(&switched));

        let list = SseSubscription {
            task_id: None,
            follow_current: false,
        };
        assert!(list.accepts(&tasks));
        assert!(!list.accepts(&todos_for_1));
    }
}
