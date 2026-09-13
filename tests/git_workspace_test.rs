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
    run(&["config", "user.email", "track@test"]);
    run(&["config", "user.name", "track"]);
    run(&["config", "commit.gpgsign", "false"]);
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
        .add_scrap_with(
            task.id,
            "chose git notes",
            track::models::ScrapVisibility::Shared,
        )
        .unwrap();
    let rev = TaskRevisionService::new(&db)
        .get(task.id, repo_str)
        .unwrap()
        .expect("task revision");
    track::use_cases::ProjectTaskNotesUseCase::new(&db)
        .execute(task.id)
        .unwrap();
    let marker_notes = git_notes::read_notes(&path, &rev.git_commit)
        .unwrap()
        .expect("marker notes");
    assert!(marker_notes.contains("Notes task"));
    assert!(
        !marker_notes.contains("chose git notes"),
        "scraps belong on the TODO commit, not the marker: {marker_notes}"
    );

    track::use_cases::CompleteTodoUseCase::new(&db)
        .execute(task.id, track::models::TodoIndex::from_i64(1))
        .unwrap();
    let todo_commits =
        track::services::todo_commit::list_todo_commits(&path, &rev.git_commit).unwrap();
    assert_eq!(todo_commits.len(), 1);
    let todo_notes = git_notes::read_notes(&path, &todo_commits[0].sha)
        .unwrap()
        .expect("todo notes");
    assert!(todo_notes.contains("chose git notes"));
    assert!(todo_notes.contains("Implement"));
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
        .add_scrap_with(
            task.id,
            "note on marker",
            track::models::ScrapVisibility::Shared,
        )
        .unwrap();
    track::use_cases::CompleteTodoUseCase::new(&db)
        .execute(task.id, track::models::TodoIndex::from_i64(1))
        .unwrap();
    let todo_commits =
        track::services::todo_commit::list_todo_commits(&path, &marker.git_commit).unwrap();
    assert_eq!(todo_commits.len(), 1);
    assert_ne!(todo_commits[0].sha, marker.git_commit);
    let on_todo = git_notes::read_notes(&path, &todo_commits[0].sha)
        .unwrap()
        .unwrap();
    assert!(on_todo.contains("note on marker"));
    let on_marker = git_notes::read_notes(&path, &marker.git_commit)
        .unwrap()
        .unwrap();
    assert!(
        !on_marker.contains("note on marker"),
        "marker notes must not hold TODO scraps: {on_marker}"
    );
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

#[test]
fn import_replays_task_from_notes_without_author_db() {
    if !git_available() {
        return;
    }
    let tmp = TempDir::new().unwrap();
    let repo = tmp.path().join("repo");
    init_git_repo(&repo);
    let repo_str = repo.to_str().unwrap();

    let author = Database::new_in_memory().unwrap();
    author.set_vcs_mode(VcsMode::Git).unwrap();
    author.set_aggressive_mode(AggressiveMode::On).unwrap();

    let task = TaskService::new(&author)
        .create_task("Replay me", Some("handoff notes"), Some("REP-1"), None)
        .unwrap();
    TodoService::new(&author)
        .add_todo(task.id, "Done item", false)
        .unwrap();
    TodoService::new(&author)
        .add_todo(task.id, "Follow up", false)
        .unwrap();
    RepoService::new(&author)
        .add_repo(task.id, repo_str, Some("main".into()), None)
        .unwrap();
    SyncTaskUseCase::new(&author)
        .execute(task.id, false)
        .unwrap();

    ScrapService::new(&author)
        .add_scrap(task.id, "private scratch")
        .unwrap();
    ScrapService::new(&author)
        .add_scrap_with(
            task.id,
            "chose git notes for replay",
            track::models::ScrapVisibility::Shared,
        )
        .unwrap();
    track::use_cases::CompleteTodoUseCase::new(&author)
        .execute(task.id, track::models::TodoIndex::from_i64(1))
        .unwrap();

    let workspace = git_worktree::git_worktree_path(repo_str, "rep-1");
    let marker = TaskRevisionService::new(&author)
        .get(task.id, repo_str)
        .unwrap()
        .unwrap();
    let first_sha = track::services::todo_commit::list_todo_commits(&workspace, &marker.git_commit)
        .unwrap()
        .into_iter()
        .next()
        .expect("first TODO commit")
        .sha;

    ScrapService::new(&author)
        .add_scrap_with(
            task.id,
            "review follow-up note",
            track::models::ScrapVisibility::Shared,
        )
        .unwrap();
    track::use_cases::CompleteTodoUseCase::new(&author)
        .execute(task.id, track::models::TodoIndex::from_i64(2))
        .unwrap();
    let after =
        track::services::todo_commit::list_todo_commits(&workspace, &marker.git_commit).unwrap();
    assert_eq!(after.len(), 2);
    assert_eq!(
        after[0].sha, first_sha,
        "follow-up must append, not rewrite"
    );

    let reader = Database::new_in_memory().unwrap();
    let outcome = track::use_cases::ImportTaskNotesUseCase::new(&reader)
        .execute(std::path::Path::new(&workspace))
        .unwrap();

    assert_eq!(outcome.task.name, "Replay me");
    assert_eq!(outcome.task.description.as_deref(), Some("handoff notes"));
    assert_eq!(outcome.todos, 2);
    assert_eq!(outcome.scraps, 2);
    let todos = TodoService::new(&reader)
        .list_todos(outcome.task.id)
        .unwrap();
    assert_eq!(todos[0].status.as_str(), "done");
    assert_eq!(todos[0].content, "Done item");
    assert_eq!(todos[1].status.as_str(), "done");
    assert_eq!(todos[1].content, "Follow up");
    let scraps = ScrapService::new(&reader)
        .list_scraps(outcome.task.id)
        .unwrap();
    assert!(
        scraps
            .iter()
            .any(|s| s.content == "chose git notes for replay")
    );
    assert!(scraps.iter().any(|s| s.content == "review follow-up note"));
    assert!(
        !scraps.iter().any(|s| s.content.contains("private")),
        "local scraps must not be imported"
    );
}

#[test]
fn follow_up_appends_after_published_todo_commit() {
    if !git_available() {
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
        .create_task("Append", None, Some("APP-1"), None)
        .unwrap();
    TodoService::new(&db)
        .add_todo(task.id, "First", false)
        .unwrap();
    TodoService::new(&db)
        .add_todo(task.id, "Review fix", false)
        .unwrap();
    RepoService::new(&db)
        .add_repo(task.id, repo_str, Some("main".into()), None)
        .unwrap();
    SyncTaskUseCase::new(&db).execute(task.id, false).unwrap();

    ScrapService::new(&db)
        .add_scrap_with(
            task.id,
            "first decision",
            track::models::ScrapVisibility::Shared,
        )
        .unwrap();
    track::use_cases::CompleteTodoUseCase::new(&db)
        .execute(task.id, track::models::TodoIndex::from_i64(1))
        .unwrap();

    let workspace = git_worktree::git_worktree_path(repo_str, "app-1");
    let marker = TaskRevisionService::new(&db)
        .get(task.id, repo_str)
        .unwrap()
        .unwrap();
    let first = git_worktree::current_commit(&workspace).unwrap();
    let first_notes = git_notes::read_notes(&workspace, &first)
        .unwrap()
        .expect("first notes");
    git_in(
        std::path::Path::new(&workspace),
        &["update-ref", "refs/remotes/origin/track/app-1", &first],
    );

    ScrapService::new(&db)
        .add_scrap_with(
            task.id,
            "review reply",
            track::models::ScrapVisibility::Shared,
        )
        .unwrap();
    track::use_cases::CompleteTodoUseCase::new(&db)
        .execute(task.id, track::models::TodoIndex::from_i64(2))
        .unwrap();

    let commits =
        track::services::todo_commit::list_todo_commits(&workspace, &marker.git_commit).unwrap();
    assert_eq!(commits.len(), 2);
    assert_eq!(commits[0].sha, first);
    assert_ne!(commits[1].sha, first);
    assert!(git_notes::is_ancestor(&workspace, &first, &commits[1].sha));
    assert_eq!(
        git_notes::read_notes(&workspace, &first)
            .unwrap()
            .as_deref(),
        Some(first_notes.as_str()),
        "published notes must not be rewritten"
    );
    let second_notes = git_notes::read_notes(&workspace, &commits[1].sha)
        .unwrap()
        .expect("follow-up notes");
    assert!(second_notes.contains("review reply"));
    assert!(!first_notes.contains("review reply"));
}

#[test]
fn follow_up_does_not_fold_published_wip() {
    if !git_available() {
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
        .create_task("Published", None, Some("PUB-1"), None)
        .unwrap();
    TodoService::new(&db)
        .add_todo(task.id, "First", false)
        .unwrap();
    TodoService::new(&db)
        .add_todo(task.id, "Review fix", false)
        .unwrap();
    RepoService::new(&db)
        .add_repo(task.id, repo_str, Some("main".into()), None)
        .unwrap();
    SyncTaskUseCase::new(&db).execute(task.id, false).unwrap();

    track::use_cases::CompleteTodoUseCase::new(&db)
        .execute(task.id, track::models::TodoIndex::from_i64(1))
        .unwrap();

    let workspace = git_worktree::git_worktree_path(repo_str, "pub-1");
    let first = git_worktree::current_commit(&workspace).unwrap();
    git_in(
        std::path::Path::new(&workspace),
        &["update-ref", "refs/remotes/origin/track/pub-1", &first],
    );

    std::fs::write(std::path::Path::new(&workspace).join("wip.txt"), "pushed\n").unwrap();
    git_in(std::path::Path::new(&workspace), &["add", "wip.txt"]);
    git_in(
        std::path::Path::new(&workspace),
        &["commit", "-m", "reviewer-visible wip"],
    );
    let published_wip = git_worktree::current_commit(&workspace).unwrap();
    git_in(
        std::path::Path::new(&workspace),
        &[
            "update-ref",
            "refs/remotes/origin/track/pub-1",
            &published_wip,
        ],
    );

    let err = track::use_cases::CompleteTodoUseCase::new(&db)
        .execute(task.id, track::models::TodoIndex::from_i64(2))
        .unwrap_err();
    assert!(
        matches!(
            err,
            track::utils::TrackError::CannotRewritePublishedHistory { .. }
        ),
        "{err}"
    );
    assert_eq!(
        git_worktree::current_commit(&workspace).unwrap(),
        published_wip
    );
}
