//! Install and refresh the managed Track section in AGENTS.md files.

use crate::utils::{Result, TrackError};
use std::fs;
use std::path::{Path, PathBuf};

pub const MARKER_START: &str = "<!-- track:agents:start v1 -->";
pub const MARKER_END: &str = "<!-- track:agents:end -->";

const SECTION_BODY: &str = include_str!("../../templates/agents-track.md");

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentsInstallScope {
    Global,
    Project,
    Custom,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AgentsInstallAction {
    Created,
    Updated,
    Unchanged,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentsInstallReport {
    pub path: PathBuf,
    pub action: AgentsInstallAction,
    pub scope: AgentsInstallScope,
}

pub fn managed_section() -> String {
    format!(
        "{MARKER_START}\n{SECTION_BODY}\n{MARKER_END}",
        SECTION_BODY = SECTION_BODY.trim_end()
    )
}

pub fn default_global_agents_path() -> Result<PathBuf> {
    let home = directories::UserDirs::new()
        .map(|dirs| dirs.home_dir().to_path_buf())
        .ok_or_else(|| TrackError::Other("could not resolve home directory".into()))?;
    Ok(home.join(".agents").join("AGENTS.md"))
}

pub fn default_project_agents_path() -> PathBuf {
    PathBuf::from("AGENTS.md")
}

pub fn resolve_agents_path(
    scope: AgentsInstallScope,
    custom_path: Option<&Path>,
) -> Result<PathBuf> {
    match scope {
        AgentsInstallScope::Global => default_global_agents_path(),
        AgentsInstallScope::Project => Ok(default_project_agents_path()),
        AgentsInstallScope::Custom => custom_path
            .map(Path::to_path_buf)
            .ok_or_else(|| TrackError::Other("--path is required for custom scope".into())),
    }
}

pub fn merge_managed_section(existing: &str, section: &str) -> (String, AgentsInstallAction) {
    if let Some((before, after_start)) = existing.split_once(MARKER_START) {
        if let Some((_, after)) = after_start.split_once(MARKER_END) {
            let rebuilt = format!(
                "{before}{section}{after}",
                before = normalize_prefix(before),
                section = section,
                after = normalize_suffix(after),
            );
            let action = if rebuilt == existing {
                AgentsInstallAction::Unchanged
            } else {
                AgentsInstallAction::Updated
            };
            return (rebuilt, action);
        }
    }

    if existing.trim().is_empty() {
        return (section.to_string(), AgentsInstallAction::Created);
    }

    let appended = format!(
        "{existing}{separator}{section}",
        existing = existing.trim_end(),
        separator = if existing.ends_with('\n') {
            "\n"
        } else {
            "\n\n"
        },
        section = section,
    );
    (appended, AgentsInstallAction::Updated)
}

pub fn install_agents_section(
    path: &Path,
    dry_run: bool,
    scope: AgentsInstallScope,
) -> Result<AgentsInstallReport> {
    let section = managed_section();
    let existing = if path.exists() {
        fs::read_to_string(path)?
    } else {
        String::new()
    };

    let (merged, action) = merge_managed_section(&existing, &section);
    if !dry_run && action != AgentsInstallAction::Unchanged {
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                fs::create_dir_all(parent)?;
            }
        }
        fs::write(path, merged)?;
    }

    Ok(AgentsInstallReport {
        path: path.to_path_buf(),
        action,
        scope,
    })
}

fn normalize_prefix(before: &str) -> String {
    if before.is_empty() {
        String::new()
    } else if before.ends_with('\n') {
        before.to_string()
    } else {
        format!("{before}\n")
    }
}

fn normalize_suffix(after: &str) -> String {
    if after.is_empty() {
        String::new()
    } else if after.starts_with('\n') {
        after.to_string()
    } else {
        format!("\n{after}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn managed_section_contains_markers() {
        let section = managed_section();
        assert!(section.contains(MARKER_START));
        assert!(section.contains(MARKER_END));
        assert!(section.contains("track status --json"));
    }

    #[test]
    fn merge_creates_new_content() {
        let (merged, action) = merge_managed_section("", &managed_section());
        assert_eq!(action, AgentsInstallAction::Created);
        assert!(merged.contains(MARKER_START));
    }

    #[test]
    fn merge_appends_when_markers_missing() {
        let existing = "# My global rules\n\nAlways run tests.\n";
        let (merged, action) = merge_managed_section(existing, &managed_section());
        assert_eq!(action, AgentsInstallAction::Updated);
        assert!(merged.starts_with("# My global rules"));
        assert!(merged.contains(MARKER_START));
    }

    #[test]
    fn merge_replaces_managed_block_idempotently() {
        let first = merge_managed_section("", &managed_section()).0;
        let (second, action) = merge_managed_section(&first, &managed_section());
        assert_eq!(action, AgentsInstallAction::Unchanged);
        assert_eq!(first, second);
    }

    #[test]
    fn merge_updates_existing_managed_block() {
        let original = format!(
            "# Rules\n\n{}\n\n## Old track section\n{}\n",
            MARKER_START, MARKER_END
        );
        let (merged, action) = merge_managed_section(&original, &managed_section());
        assert_eq!(action, AgentsInstallAction::Updated);
        assert!(merged.contains("track status --json"));
        assert!(!merged.contains("Old track section"));
    }

    #[test]
    fn install_writes_global_file() {
        let temp = TempDir::new().unwrap();
        let path = temp.path().join(".agents").join("AGENTS.md");

        let report = install_agents_section(&path, false, AgentsInstallScope::Global).unwrap();
        assert_eq!(report.action, AgentsInstallAction::Created);
        let content = fs::read_to_string(&path).unwrap();
        assert!(content.contains(MARKER_START));
    }

    #[test]
    fn dry_run_does_not_write() {
        let temp = TempDir::new().unwrap();
        let path = temp.path().join("AGENTS.md");

        let report = install_agents_section(&path, true, AgentsInstallScope::Project).unwrap();
        assert_eq!(report.action, AgentsInstallAction::Created);
        assert!(!path.exists());
    }
}
