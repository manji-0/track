use serde::Deserialize;
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Deserialize)]
pub(super) struct JjTaskMap {
    #[serde(default)]
    pub repos: HashMap<String, JjRepoEntry>,
}

#[derive(Debug, Deserialize)]
pub(super) struct JjRepoEntry {
    #[serde(default)]
    pub tasks: HashMap<String, JjTaskEntry>,
}

#[derive(Debug, Deserialize)]
pub(super) struct JjTaskEntry {
    pub workspace: String,
    #[serde(default)]
    pub phase: String,
}

pub(super) fn map_path() -> PathBuf {
    std::env::var("JJ_TASK_MAP")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            directories::BaseDirs::new()
                .map(|dirs| dirs.home_dir().join(".config/jj/task-workspaces.json"))
                .unwrap_or_else(|| PathBuf::from(".config/jj/task-workspaces.json"))
        })
}

/// Loads the jj-task workspace map. Returns `None` when the file is missing.
/// Logs a warning when the file exists but cannot be parsed.
pub(super) fn load_map() -> Option<JjTaskMap> {
    let path = map_path();
    let content = match std::fs::read_to_string(&path) {
        Ok(content) => content,
        Err(_) => return None,
    };

    match serde_json::from_str(&content) {
        Ok(map) => Some(map),
        Err(err) => {
            eprintln!("warning: invalid jj-task map at {}: {err}", path.display());
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invalid_map_file_returns_none() {
        let temp = tempfile::tempdir().unwrap();
        let map_path = temp.path().join("task-workspaces.json");
        std::fs::write(&map_path, "{not-json").unwrap();

        let prev = std::env::var("JJ_TASK_MAP").ok();
        unsafe { std::env::set_var("JJ_TASK_MAP", &map_path) };

        assert!(load_map().is_none());

        match prev {
            Some(value) => unsafe { std::env::set_var("JJ_TASK_MAP", value) },
            None => unsafe { std::env::remove_var("JJ_TASK_MAP") },
        }
    }
}
