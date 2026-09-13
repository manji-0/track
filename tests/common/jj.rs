//! Isolated jj repositories for integration tests.
//!
//! Each [`JjWorkspace`] uses a private `HOME` and temp directory so tests do not
//! depend on the developer's jj configuration or working copy state.

use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Mutex, MutexGuard, OnceLock};
use tempfile::TempDir;

static JJ_ENV_MUTEX: OnceLock<Mutex<()>> = OnceLock::new();
static CWD_MUTEX: OnceLock<Mutex<()>> = OnceLock::new();

/// Serializes jj integration tests that mutate process-global state (`HOME`).
pub fn jj_test_lock() -> MutexGuard<'static, ()> {
    JJ_ENV_MUTEX.get_or_init(|| Mutex::new(())).lock().unwrap()
}

/// Serializes tests that change the process current directory.
pub fn cwd_lock() -> MutexGuard<'static, ()> {
    CWD_MUTEX.get_or_init(|| Mutex::new(())).lock().unwrap()
}

/// Temporary jj environment with an isolated `HOME`.
pub struct JjIsolation {
    _home_dir: TempDir,
    old_home: Option<OsString>,
}

impl JjIsolation {
    /// Returns `None` when the `jj` binary is not available.
    pub fn new() -> Option<Self> {
        if !jj_available() {
            return None;
        }

        let home_dir = tempfile::tempdir().expect("temp HOME");
        configure_jj_user(home_dir.path());

        let old_home = std::env::var_os("HOME");
        unsafe {
            std::env::set_var("HOME", home_dir.path());
        }

        Some(Self {
            _home_dir: home_dir,
            old_home,
        })
    }
}

impl Drop for JjIsolation {
    fn drop(&mut self) {
        restore_env_var("HOME", self.old_home.take());
    }
}

/// Virtual workspace: temp root + one clean jj repository.
pub struct JjWorkspace {
    _guard: MutexGuard<'static, ()>,
    _root: TempDir,
    _isolation: JjIsolation,
    repo_path: PathBuf,
}

impl JjWorkspace {
    /// Skip the test when `jj` is not installed.
    pub fn new() -> Option<Self> {
        let guard = jj_test_lock();
        let isolation = JjIsolation::new()?;
        let root = tempfile::tempdir().expect("temp workspace root");
        let repo_path = root.path().join("repo");
        init_clean_jj_repo(&repo_path);

        Some(Self {
            _guard: guard,
            _root: root,
            _isolation: isolation,
            repo_path,
        })
    }

    pub fn root(&self) -> &TempDir {
        &self._root
    }

    pub fn repo_path(&self) -> &Path {
        &self.repo_path
    }

    pub fn repo_path_string(&self) -> String {
        self.repo_path.to_string_lossy().into_owned()
    }

    /// Creates an additional clean jj repo under this workspace root.
    pub fn add_repo(&self, name: &str) -> PathBuf {
        let repo_path = self._root.path().join(name);
        std::fs::create_dir_all(&repo_path).expect("create nested repo dir");
        init_clean_jj_repo(&repo_path);
        repo_path
    }

    pub fn describe_change(&self, message: &str) {
        describe_change(self.repo_path(), message);
    }

    pub fn new_change(&self) {
        new_change(self.repo_path());
    }

    pub fn create_bookmark(&self, name: &str) {
        create_bookmark(self.repo_path(), name);
    }

    pub fn assert_base_clean(&self) {
        assert_base_clean(self.repo_path());
    }
}

pub fn jj_available() -> bool {
    Command::new("jj")
        .arg("--version")
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

/// Initializes a jj repo and leaves the base working copy with no pending changes.
pub fn init_clean_jj_repo(repo_path: &Path) {
    let repo_str = repo_path
        .to_str()
        .expect("repo path must be valid UTF-8 for jj tests");

    std::fs::create_dir_all(repo_path).expect("create repo directory");

    let output = Command::new("jj")
        .current_dir(repo_path)
        .args(["git", "init", repo_str])
        .output()
        .unwrap_or_else(|err| panic!("failed to run jj git init: {err}"));
    assert!(
        output.status.success(),
        "jj git init failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    std::fs::write(repo_path.join("README.md"), "track test repo\n").expect("write README");
    run_jj(repo_path, &["describe", "-m", "init"], "jj describe");
    run_jj(repo_path, &["new"], "jj new");
    assert_base_clean(repo_path);
}

pub fn describe_change(repo_path: &Path, message: &str) {
    run_jj(repo_path, &["describe", "-m", message], "jj describe");
}

pub fn new_change(repo_path: &Path) {
    run_jj(repo_path, &["new"], "jj new");
}

pub fn create_bookmark(repo_path: &Path, name: &str) {
    run_jj(
        repo_path,
        &["bookmark", "create", name, "-r", "@"],
        "jj bookmark create",
    );
}

pub fn assert_base_clean(repo_path: &Path) {
    let output = Command::new("jj")
        .current_dir(repo_path)
        .args(["-R"])
        .arg(repo_path)
        .args(["diff", "--summary"])
        .output()
        .expect("jj diff --summary");

    assert!(
        output.status.success(),
        "jj diff --summary failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let summary = String::from_utf8_lossy(&output.stdout);
    assert!(
        summary.trim().is_empty(),
        "base workspace has pending changes: {summary:?}"
    );
}

fn configure_jj_user(home: &Path) {
    for (key, value) in [
        ("user.name", "Track Test"),
        ("user.email", "test@track.local"),
    ] {
        let output = Command::new("jj")
            .env("HOME", home)
            .args(["config", "set", "--user", key, value])
            .output()
            .expect("jj config set");
        assert!(
            output.status.success(),
            "jj config set {key} failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

fn run_jj(repo_path: &Path, args: &[&str], label: &str) {
    let output = Command::new("jj")
        .current_dir(repo_path)
        .args(["-R"])
        .arg(repo_path)
        .args(args)
        .output()
        .unwrap_or_else(|err| panic!("failed to run {label}: {err}"));

    assert!(
        output.status.success(),
        "{label} failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

fn restore_env_var(key: &str, value: Option<OsString>) {
    match value {
        Some(value) => unsafe { std::env::set_var(key, value) },
        None => unsafe { std::env::remove_var(key) },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn isolated_workspace_starts_with_clean_base() {
        let Some(ws) = JjWorkspace::new() else {
            eprintln!("Skipping test: jj binary not available");
            return;
        };
        ws.assert_base_clean();
    }
}
