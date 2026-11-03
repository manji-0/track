//! Session bindings: `TRACK_TASK` and the workspace cwd act as an implicit `--task`.

use std::path::{Path, PathBuf};
use std::process::Command;
use tempfile::TempDir;
use track::cli::handler::CommandHandler;
use track::cli::{Commands, TodoCommands};
use track::db::Database;
use track::models::{ContextMode, SessionBinding, TaskId};
use track::services::{RepoService, TaskService, TodoService};
use track::utils::TrackError;

fn todo_add(text: &str) -> Commands {
    Commands::Todo(TodoCommands::Add {
        text: text.to_string(),
        worktree: false,
        no_workspace: true,
        json: false,
    })
}

fn todo_contents(db: &Database, task_id: TaskId) -> Vec<String> {
    TodoService::new(db)
        .list_todos(task_id)
        .unwrap()
        .into_iter()
        .map(|todo| todo.content)
        .collect()
}

fn env_binding(reference: &str) -> SessionBinding {
    SessionBinding {
        env_task: Some(reference.to_string()),
        cwd: None,
    }
}

fn cwd_binding(cwd: &Path) -> SessionBinding {
    SessionBinding {
        env_task: None,
        cwd: Some(cwd.to_path_buf()),
    }
}

fn handler_with_tasks(mode: ContextMode) -> (CommandHandler, TaskId, TaskId) {
    let handler = CommandHandler::from_db(Database::new_in_memory().unwrap());
    let service = TaskService::new(handler.get_db());
    let a = service.create_task("A", None, None, None).unwrap();
    let b = service.create_task("B", None, None, None).unwrap();
    service.set_alias(a.id, "aaa", false).unwrap();
    service.set_alias(b.id, "bbb", false).unwrap();
    handler.get_db().set_context_mode(mode).unwrap();
    (handler, a.id, b.id)
}

/// A fake repo (`.git` marker) registered to `task_id`, with `<repo>/.worktrees/<slug>/sub` present.
fn register_workspace(db: &Database, task_id: TaskId, slug: &str) -> (TempDir, PathBuf) {
    let repo = TempDir::new().unwrap();
    std::fs::create_dir(repo.path().join(".git")).unwrap();
    let workspace = repo.path().join(".worktrees").join(slug);
    std::fs::create_dir_all(workspace.join("sub")).unwrap();
    RepoService::new(db)
        .add_repo(task_id, repo.path().to_str().unwrap(), None, None)
        .unwrap();
    (repo, workspace)
}

#[test]
fn two_env_bindings_write_to_their_own_tasks_in_multi_mode() {
    let (handler, a, b) = handler_with_tasks(ContextMode::Multi);

    handler
        .handle_with_target(todo_add("one"), None, env_binding(&a.to_string()))
        .unwrap();
    handler
        .handle_with_target(todo_add("two"), None, env_binding("a:bbb"))
        .unwrap();
    handler
        .handle_with_target(todo_add("three"), None, env_binding(&a.to_string()))
        .unwrap();

    let db = handler.get_db();
    assert_eq!(todo_contents(db, a), vec!["one", "three"]);
    assert_eq!(todo_contents(db, b), vec!["two"]);
}

#[test]
fn empty_env_binding_counts_as_unbound() {
    let (handler, _, _) = handler_with_tasks(ContextMode::Multi);
    let err = handler
        .handle_with_target(todo_add("x"), None, env_binding(""))
        .unwrap_err();
    assert!(matches!(err, TrackError::TaskRefRequired), "{err}");
}

#[test]
fn unresolvable_env_ref_is_an_error_not_a_fallback() {
    let (handler, a, _) = handler_with_tasks(ContextMode::Single);
    let db = handler.get_db();
    db.set_current_task_id(a).unwrap();

    let err = handler
        .handle_with_target(todo_add("x"), None, env_binding("999"))
        .unwrap_err();
    assert!(matches!(err, TrackError::TaskNotFound(_)), "{err}");
    assert!(todo_contents(db, a).is_empty());
}

#[test]
fn cwd_inside_a_workspace_selects_that_task() {
    let (handler, a, b) = handler_with_tasks(ContextMode::Multi);
    let db = handler.get_db();
    let (_repo_a, ws_a) = register_workspace(db, a, "aaa");
    let (_repo_b, ws_b) = register_workspace(db, b, "bbb");

    handler
        .handle_with_target(todo_add("from a"), None, cwd_binding(&ws_a.join("sub")))
        .unwrap();
    handler
        .handle_with_target(todo_add("from b root"), None, cwd_binding(&ws_b))
        .unwrap();

    assert_eq!(todo_contents(db, a), vec!["from a"]);
    assert_eq!(todo_contents(db, b), vec!["from b root"]);
}

#[test]
fn sibling_directory_with_a_shared_name_prefix_does_not_match() {
    let (handler, a, _) = handler_with_tasks(ContextMode::Multi);
    let db = handler.get_db();
    let (repo, _ws) = register_workspace(db, a, "aaa");
    let sibling = repo.path().join(".worktrees").join("aaax");
    std::fs::create_dir_all(&sibling).unwrap();

    let err = handler
        .handle_with_target(todo_add("x"), None, cwd_binding(&sibling))
        .unwrap_err();
    assert!(matches!(err, TrackError::TaskRefRequired), "{err}");
    assert!(
        !TaskService::new(db)
            .find_task_by_workspace(repo.path())
            .unwrap()
            .is_some(),
        "the repo root is not a task workspace"
    );
}

#[test]
fn flag_beats_env_beats_workspace_beats_current() {
    let (handler, a, b) = handler_with_tasks(ContextMode::Single);
    let db = handler.get_db();
    let service = TaskService::new(db);
    let c = service.create_task("C", None, None, None).unwrap().id;
    let d = service.create_task("D", None, None, None).unwrap().id;
    let (_repo, workspace) = register_workspace(db, d, "task-4");
    db.set_current_task_id(c).unwrap();

    let both = SessionBinding {
        env_task: Some(b.to_string()),
        cwd: Some(workspace.clone()),
    };
    handler
        .handle_with_target(todo_add("flag"), Some(&a.to_string()), both.clone())
        .unwrap();
    handler
        .handle_with_target(todo_add("env"), None, both)
        .unwrap();
    handler
        .handle_with_target(todo_add("workspace"), None, cwd_binding(&workspace))
        .unwrap();
    handler
        .handle_with_target(todo_add("current"), None, SessionBinding::default())
        .unwrap();

    assert_eq!(todo_contents(db, a), vec!["flag"]);
    assert_eq!(todo_contents(db, b), vec!["env"]);
    assert_eq!(todo_contents(db, d), vec!["workspace"]);
    assert_eq!(todo_contents(db, c), vec!["current"]);
}

#[test]
fn single_mode_without_a_binding_still_uses_the_current_task() {
    let (handler, _, b) = handler_with_tasks(ContextMode::Single);
    handler.handle(todo_add("x")).unwrap();
    assert_eq!(todo_contents(handler.get_db(), b), vec!["x"]);
}

fn run_track(home: &Path, cwd: &Path, env_task: Option<&str>, args: &[&str]) -> (bool, String) {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_track"));
    cmd.env("HOME", home)
        .env("XDG_DATA_HOME", home.join("data"))
        .env("TRACK_HINTS", "0")
        .env_remove("TRACK_TASK")
        .current_dir(cwd)
        .args(args);
    if let Some(reference) = env_task {
        cmd.env("TRACK_TASK", reference);
    }
    let out = cmd.output().unwrap();
    (
        out.status.success(),
        String::from_utf8_lossy(&out.stdout).into_owned(),
    )
}

#[test]
fn status_json_reports_the_target_source() {
    let home = TempDir::new().unwrap();
    let cwd = TempDir::new().unwrap();
    let (h, c) = (home.path(), cwd.path());
    run_track(h, c, None, &["new", "One"]);
    run_track(h, c, None, &["new", "Two"]);
    run_track(h, c, None, &["config", "set", "context-mode", "multi"]);

    let source = |env: Option<&str>, args: &[&str]| {
        let (ok, out) = run_track(h, c, env, args);
        assert!(ok, "{args:?} failed");
        let value: serde_json::Value = serde_json::from_str(&out).unwrap();
        (
            value["task"]["name"].as_str().unwrap().to_string(),
            value["target_source"].as_str().unwrap().to_string(),
        )
    };

    assert_eq!(
        source(None, &["--task", "1", "status", "--json"]),
        ("One".to_string(), "flag".to_string())
    );
    assert_eq!(
        source(Some("2"), &["status", "--json"]),
        ("Two".to_string(), "env".to_string())
    );
    let (ok, _) = run_track(h, c, None, &["status", "--json"]);
    assert!(!ok, "multi mode without a binding must refuse");

    run_track(h, c, None, &["config", "set", "context-mode", "single"]);
    assert_eq!(
        source(None, &["status", "--json"]),
        ("Two".to_string(), "current".to_string())
    );
}
