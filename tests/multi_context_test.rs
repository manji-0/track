//! Multi-context mode: explicit task references in the CLI and per-tab WebUI scoping.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use std::sync::Arc;
use tower::ServiceExt;
use track::cli::handler::CommandHandler;
use track::cli::{Commands, ScrapCommands, TodoCommands};
use track::db::Database;
use track::models::{ContextMode, TaskId};
use track::services::{TaskService, TodoService};
use track::utils::TrackError;
use track::webui::{AppState, Templates, WebState, build_router};

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

#[test]
fn multi_mode_refuses_task_scoped_commands_without_reference() {
    let handler = CommandHandler::from_db(Database::new_in_memory().unwrap());
    let db = handler.get_db();
    TaskService::new(db)
        .create_task("A", None, None, None)
        .unwrap();
    db.set_context_mode(ContextMode::Multi).unwrap();

    for command in [
        todo_add("x"),
        Commands::Scrap(ScrapCommands::List),
        Commands::Status {
            id: None,
            json: true,
            all: false,
        },
        Commands::Desc { description: None },
    ] {
        let err = handler.handle(command).unwrap_err();
        assert!(matches!(err, TrackError::TaskRefRequired), "{err}");
    }
}

#[test]
fn multi_mode_routes_commands_to_the_referenced_task() {
    let handler = CommandHandler::from_db(Database::new_in_memory().unwrap());
    let db = handler.get_db();
    let task_service = TaskService::new(db);
    let a = task_service
        .create_task("Task A", None, None, None)
        .unwrap();
    let b = task_service
        .create_task("Task B", None, None, None)
        .unwrap();
    task_service.set_alias(b.id, "bee", false).unwrap();
    db.set_context_mode(ContextMode::Multi).unwrap();

    handler
        .handle_with_task(todo_add("for A"), Some(&a.id.to_string()))
        .unwrap();
    handler
        .handle_with_task(todo_add("for B by alias"), Some("a:bee"))
        .unwrap();
    handler
        .handle_with_task(todo_add("for A by name"), Some("Task A"))
        .unwrap();

    assert_eq!(todo_contents(db, a.id), vec!["for A", "for A by name"]);
    assert_eq!(todo_contents(db, b.id), vec!["for B by alias"]);
}

#[test]
fn multi_mode_disables_switch() {
    let handler = CommandHandler::from_db(Database::new_in_memory().unwrap());
    let db = handler.get_db();
    let task = TaskService::new(db)
        .create_task("A", None, None, None)
        .unwrap();
    db.set_context_mode(ContextMode::Multi).unwrap();

    let err = handler
        .handle(Commands::Switch {
            task_ref: task.id.to_string(),
            json: false,
        })
        .unwrap_err();
    assert!(matches!(err, TrackError::SwitchUnavailableInMultiContext));
}

#[test]
fn single_mode_task_flag_overrides_current_task() {
    let handler = CommandHandler::from_db(Database::new_in_memory().unwrap());
    let db = handler.get_db();
    let task_service = TaskService::new(db);
    let a = task_service.create_task("A", None, None, None).unwrap();
    let b = task_service.create_task("B", None, None, None).unwrap();
    assert_eq!(db.get_current_task_id().unwrap(), Some(b.id));

    handler.handle(todo_add("current")).unwrap();
    handler
        .handle_with_task(todo_add("explicit"), Some(&a.id.to_string()))
        .unwrap();

    assert_eq!(todo_contents(db, a.id), vec!["explicit"]);
    assert_eq!(todo_contents(db, b.id), vec!["current"]);
    assert_eq!(db.get_current_task_id().unwrap(), Some(b.id));
}

fn test_router(db: Database) -> axum::Router {
    build_router(WebState {
        app: AppState::from_database(db),
        templates: Arc::new(Templates::embedded()),
    })
}

async fn send(app: &axum::Router, request: Request<Body>) -> (StatusCode, String) {
    let response = app.clone().oneshot(request).await.unwrap();
    let status = response.status();
    let body = http_body_util::BodyExt::collect(response.into_body())
        .await
        .unwrap()
        .to_bytes();
    (status, String::from_utf8(body.to_vec()).unwrap())
}

fn get(uri: &str, task: Option<TaskId>) -> Request<Body> {
    let mut builder = Request::builder().uri(uri);
    if let Some(id) = task {
        builder = builder.header("X-Track-Task", id.to_string());
    }
    builder.body(Body::empty()).unwrap()
}

fn post_todo(task: Option<TaskId>, content: &str) -> Request<Body> {
    let mut builder = Request::builder()
        .method("POST")
        .uri("/api/todo")
        .header("content-type", "application/x-www-form-urlencoded");
    if let Some(id) = task {
        builder = builder.header("X-Track-Task", id.to_string());
    }
    builder
        .body(Body::from(format!("content={content}&no_workspace=true")))
        .unwrap()
}

/// Two tasks, multi mode, one TODO each.
fn two_task_db() -> (Database, TaskId, TaskId) {
    let db = Database::new_in_memory().unwrap();
    let task_service = TaskService::new(&db);
    let a = task_service.create_task("Tab A", None, None, None).unwrap();
    let b = task_service.create_task("Tab B", None, None, None).unwrap();
    let todo_service = TodoService::new(&db);
    todo_service.add_todo(a.id, "todo-of-a", true).unwrap();
    todo_service.add_todo(b.id, "todo-of-b", true).unwrap();
    db.set_context_mode(ContextMode::Multi).unwrap();
    (db, a.id, b.id)
}

#[tokio::test]
async fn webui_serves_each_task_to_its_own_tab() {
    let (db, a, b) = two_task_db();
    let app = test_router(db);

    let (status, page_a) = send(&app, get(&format!("/tasks/{a}"), None)).await;
    assert_eq!(status, StatusCode::OK);
    assert!(
        page_a.contains(&format!(r#"data-task-id="{a}""#)),
        "{page_a}"
    );
    assert!(page_a.contains("todo-of-a"));
    assert!(!page_a.contains("todo-of-b"));
    assert!(page_a.contains(&format!("/api/sse?task={a}&amp;follow=false")));

    let (_, todos_b) = send(&app, get("/partials/todos", Some(b))).await;
    assert!(todos_b.contains("todo-of-b"));
    assert!(!todos_b.contains("todo-of-a"));

    let (_, status_a) = send(&app, get(&format!("/api/status?task={a}"), None)).await;
    let json: serde_json::Value = serde_json::from_str(&status_a).unwrap();
    assert_eq!(json["task"]["name"], "Tab A");
    assert_eq!(json["context_mode"], "multi");
}

#[tokio::test]
async fn webui_writes_land_on_the_tab_task() {
    let (db, a, b) = two_task_db();
    let app = test_router(db);

    let (status, html) = send(&app, post_todo(Some(a), "from-tab-a")).await;
    assert_eq!(status, StatusCode::OK);
    assert!(html.contains("from-tab-a"));

    let (_, todos_b) = send(&app, get("/partials/todos", Some(b))).await;
    assert!(!todos_b.contains("from-tab-a"));
}

#[tokio::test]
async fn webui_multi_mode_rejects_requests_without_task() {
    let (db, _, _) = two_task_db();
    let app = test_router(db);

    let (status, _) = send(&app, post_todo(None, "orphan")).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);

    let (status, _) = send(&app, get("/api/status", None)).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn webui_index_lists_tasks_in_multi_mode() {
    let (db, a, b) = two_task_db();
    let app = test_router(db);

    let (status, html) = send(&app, get("/", None)).await;
    assert_eq!(status, StatusCode::OK);
    assert!(html.contains(&format!(r#"href="/tasks/{a}""#)), "{html}");
    assert!(html.contains(&format!(r#"href="/tasks/{b}""#)));
    assert!(!html.contains("data-task-id"));
}

#[tokio::test]
async fn webui_index_follows_current_task_in_single_mode() {
    let (db, _, b) = two_task_db();
    db.set_context_mode(ContextMode::Single).unwrap();
    let app = test_router(db);

    let (_, html) = send(&app, get("/", None)).await;
    assert!(html.contains(&format!(r#"data-task-id="{b}""#)), "{html}");
    assert!(html.contains("follow=true"), "{html}");
}
