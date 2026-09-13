//! Git worktree + aggressive-mode git notes.

use std::process::Command;
use tempfile::TempDir;
use track::db::Database;
use track::models::{AggressiveMode, VcsMode};
use track::services::{
    RepoService, ScrapService, TaskRevisionService, TaskService, TodoService, git_notes,
    git_worktree, task_workspace,
};
use track::use_cases::SyncTaskUseCase;

fn git_available() -> bool {
    Command::new("git")
        .arg("--version")
        .output()
        .is_ok_and(|o| o.status.success())
}

fn init_git_repo(path: &std::path::Path) {
    std::fs::create_dir_all(path).unwrap();
    let run = |args: &[&str]| {
        let output = Command::new("git")
            .current_dir(path)
            .args([
                "-c",
                "user.email=track@test",
                "-c",
                "user.name=track",
                "-c",
                "commit.gpgsign=false",
            ])
            .args(args)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "git {:?} failed: {}",
            args,
            String::from_utf8_lossy(&output.stderr)
        );
    };
    run(&["init", "-b", "main"]);
    std::fs::write(path.join("README.md"), "hello\n").unwrap();
    run(&["add", "README.md"]);
    run(&["commit", "-m", "init"]);
}

#[test]
fn git_sync_creates_worktree_and_aggressive_notes() {
    if !git_available() {
        eprintln!("Skipping test: git binary not available");
        return;
    }

    let tmp = TempDir::new().unwrap();
    let repo = tmp.path().join("repo");
    init_git_repo(&repo);
    let repo_str = repo.to_str().unwrap();

    let db = Database::new_in_memory().unwrap();
    db.set_vcs_mode(VcsMode::Git).unwrap();
    db.set_aggressive_mode(AggressiveMode::On).unwrap();

    let task = TaskService::new(&db)
        .create_task("Notes task", None, Some("NOTE-1"), None)
        .unwrap();
    TodoService::new(&db)
        .add_todo(task.id, "Implement", false)
        .unwrap();
    RepoService::new(&db)
        .add_repo(task.id, repo_str, Some("main".into()), None)
        .unwrap();

    let outcome = SyncTaskUseCase::new(&db).execute(task.id, false).unwrap();
    let path = git_worktree::git_worktree_path(repo_str, "note-1");
    assert!(
        outcome
            .repos
            .iter()
            .any(|(_, o)| matches!(o, track::use_cases::RepoSyncOutcome::WorktreeCreated { .. }))
    );
    assert!(std::path::Path::new(&path).is_dir());
    assert!(git_worktree::branch_exists(repo_str, "refs/heads/track/note-1").unwrap());

    ScrapService::new(&db)
        .add_scrap(task.id, "chose git notes")
        .unwrap();
    let rev = TaskRevisionService::new(&db)
        .get(task.id, repo_str)
        .unwrap()
        .expect("task revision");
    git_notes::write_scraps(
        &path,
        &rev.git_commit,
        "note-1",
        &ScrapService::new(&db).list_scraps(task.id).unwrap(),
    )
    .unwrap();
    let notes = git_notes::read_notes(&path, &rev.git_commit)
        .unwrap()
        .expect("notes");
    assert!(notes.contains("chose git notes"));
    assert_eq!(task_workspace::branch_name("note-1"), "track/note-1");
    assert!(
        !repo.join(".gitignore").exists(),
        "track must not dirty the project .gitignore"
    );
}

fn git_in(path: &std::path::Path, args: &[&str]) {
    let output = Command::new("git")
        .current_dir(path)
        .args([
            "-c",
            "user.email=track@test",
            "-c",
            "user.name=track",
            "-c",
            "commit.gpgsign=false",
        ])
        .args(args)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "git {:?} failed: {}",
        args,
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn git_sync_does_not_block_second_task_on_ignore_file() {
    if !git_available() {
        return;
    }
    let tmp = TempDir::new().unwrap();
    let repo = tmp.path().join("repo");
    init_git_repo(&repo);
    let repo_str = repo.to_str().unwrap();

    let db = Database::new_in_memory().unwrap();
    db.set_vcs_mode(VcsMode::Git).unwrap();

    let task_a = TaskService::new(&db)
        .create_task("A", None, Some("A-1"), None)
        .unwrap();
    TodoService::new(&db)
        .add_todo(task_a.id, "Do A", false)
        .unwrap();
    RepoService::new(&db)
        .add_repo(task_a.id, repo_str, Some("main".into()), None)
        .unwrap();
    SyncTaskUseCase::new(&db).execute(task_a.id, false).unwrap();

    let task_b = TaskService::new(&db)
        .create_task("B", None, Some("B-1"), None)
        .unwrap();
    db.set_current_task_id(task_b.id).unwrap();
    TodoService::new(&db)
        .add_todo(task_b.id, "Do B", false)
        .unwrap();
    RepoService::new(&db)
        .add_repo(task_b.id, repo_str, Some("main".into()), None)
        .unwrap();
    let outcome = SyncTaskUseCase::new(&db).execute(task_b.id, false).unwrap();
    assert!(
        outcome
            .repos
            .iter()
            .any(|(_, o)| matches!(o, track::use_cases::RepoSyncOutcome::WorktreeCreated { .. })),
        "second task workspace must not be blocked by local exclude: {outcome:?}"
    );
}

#[test]
fn aggressive_backfill_and_marker_is_immutable() {
    if !git_available() {
        return;
    }
    let tmp = TempDir::new().unwrap();
    let repo = tmp.path().join("repo");
    init_git_repo(&repo);
    let repo_str = repo.to_str().unwrap();

    let db = Database::new_in_memory().unwrap();
    db.set_vcs_mode(VcsMode::Git).unwrap();
    db.set_aggressive_mode(AggressiveMode::Off).unwrap();

    let task = TaskService::new(&db)
        .create_task("Later", None, Some("LAT-1"), None)
        .unwrap();
    TodoService::new(&db)
        .add_todo(task.id, "Implement", false)
        .unwrap();
    RepoService::new(&db)
        .add_repo(task.id, repo_str, Some("main".into()), None)
        .unwrap();
    SyncTaskUseCase::new(&db).execute(task.id, false).unwrap();
    assert!(
        TaskRevisionService::new(&db)
            .get(task.id, repo_str)
            .unwrap()
            .is_none()
    );

    db.set_aggressive_mode(AggressiveMode::On).unwrap();
    SyncTaskUseCase::new(&db).execute(task.id, false).unwrap();
    let marker = TaskRevisionService::new(&db)
        .get(task.id, repo_str)
        .unwrap()
        .expect("backfilled marker");

    let path = git_worktree::git_worktree_path(repo_str, "lat-1");
    std::fs::write(std::path::Path::new(&path).join("work.txt"), "w\n").unwrap();
    git_in(std::path::Path::new(&path), &["add", "work.txt"]);
    git_in(std::path::Path::new(&path), &["commit", "-m", "real work"]);

    SyncTaskUseCase::new(&db).execute(task.id, false).unwrap();
    let again = TaskRevisionService::new(&db)
        .get(task.id, repo_str)
        .unwrap()
        .unwrap();
    assert_eq!(again.git_commit, marker.git_commit);

    let head = git_worktree::current_commit(&path).unwrap();
    assert_ne!(head, marker.git_commit);

    ScrapService::new(&db)
        .add_scrap(task.id, "note on marker")
        .unwrap();
    git_notes::write_scraps(
        &path,
        &marker.git_commit,
        "lat-1",
        &ScrapService::new(&db).list_scraps(task.id).unwrap(),
    )
    .unwrap();
    let on_marker = git_notes::read_notes(&path, &marker.git_commit)
        .unwrap()
        .unwrap();
    assert!(on_marker.contains("note on marker"));
    assert!(
        git_notes::read_notes(&path, &head)
            .unwrap()
            .is_none_or(|n| !n.contains("note on marker"))
    );
}

#[test]
fn git_workspace_rejects_jj_mode() {
    if !git_available() {
        return;
    }
    let tmp = TempDir::new().unwrap();
    let repo = tmp.path().join("repo");
    init_git_repo(&repo);
    let repo_str = repo.to_str().unwrap();

    let db = Database::new_in_memory().unwrap();
    db.set_vcs_mode(VcsMode::Git).unwrap();
    let task = TaskService::new(&db)
        .create_task("Mix", None, Some("MIX-1"), None)
        .unwrap();
    TodoService::new(&db)
        .add_todo(task.id, "Implement", false)
        .unwrap();
    RepoService::new(&db)
        .add_repo(task.id, repo_str, Some("main".into()), None)
        .unwrap();
    SyncTaskUseCase::new(&db).execute(task.id, false).unwrap();

    db.set_vcs_mode(VcsMode::Jj).unwrap();
    let err = SyncTaskUseCase::new(&db)
        .execute(task.id, false)
        .unwrap_err();
    assert!(
        matches!(err, track::utils::TrackError::WorkspaceVcsMismatch { .. }),
        "{err}"
    );
}
