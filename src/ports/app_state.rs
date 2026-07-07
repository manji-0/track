//! Application state persistence port.

use crate::utils::Result;

/// Read/write session state stored in SQLite `app_state`.
pub trait AppStateStore {
    fn get_current_task_id(&self) -> Result<Option<i64>>;
    fn set_current_task_id(&self, task_id: i64) -> Result<()>;
    fn clear_current_task_id(&self) -> Result<()>;
}
