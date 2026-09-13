use axum::body::Body;
use axum::http::{Request, StatusCode};
use std::sync::Arc;
use tower::ServiceExt;
use track::db::Database;
use track::models::TodoStatus;
use track::services::{TaskService, TodoService};
use track::webui::{AppState, Templates, WebState, build_router};

fn test_router(db: Database) -> axum::Router {
    let app_state = AppState::from_database(db);
    let web_state = WebState {
        app: app_state,
        templates: Arc::new(Templates::embedded()),
    };
    build_router(web_state)
}

#[tokio::test]
async fn api_status_without_active_task_returns_empty_payload() {
    let db = Database::new_in_memory().unwrap();
    let app = test_router(db);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/status")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = http_body_util::BodyExt::collect(response.into_body())
        .await
        .unwrap()
        .to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert!(json.get("task").unwrap().is_null());
    assert_eq!(json.get("todos").unwrap().as_array().unwrap().len(), 0);
}

#[tokio::test]
async fn api_status_returns_current_task_todos() {
    let db = Database::new_in_memory().unwrap();
    let task_service = TaskService::new(&db);
    let task = task_service
        .create_task("Web task", None, None, None)
        .unwrap();
    db.set_current_task_id(task.id).unwrap();

    let todo_service = TodoService::new(&db);
    todo_service
        .add_todo(task.id, "From browser", false)
        .unwrap();

    let app = test_router(db);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/status")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = http_body_util::BodyExt::collect(response.into_body())
        .await
        .unwrap()
        .to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json.get("task").unwrap()["name"], "Web task");
    assert_eq!(json.get("todos").unwrap().as_array().unwrap().len(), 1);
}

#[tokio::test]
async fn api_status_includes_workflow_fields() {
    let db = Database::new_in_memory().unwrap();
    let task_service = TaskService::new(&db);
    let task = task_service
        .create_task("Web task", None, None, None)
        .unwrap();
    db.set_current_task_id(task.id).unwrap();

    let todo_service = TodoService::new(&db);
    todo_service
        .add_todo(task.id, "From browser", false)
        .unwrap();

    let app = test_router(db);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/status")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = http_body_util::BodyExt::collect(response.into_body())
        .await
        .unwrap()
        .to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["workflow"]["phase"], "setup");
    assert_eq!(json["vcs_mode"], "git");
    assert!(json["guardrails"]["reopen_forbidden"].as_bool().unwrap());
    assert!(!json["guardrails"]["must_use_jj_skill"].as_bool().unwrap());
    assert!(json.get("jj").is_none());
    assert_eq!(
        json["git"]["branch"],
        format!("track/task-{}", json["task"]["id"])
    );
    assert_eq!(json["todos_agent"].as_array().unwrap().len(), 1);
}

#[tokio::test]
async fn add_todo_rejects_empty_content() {
    let db = Database::new_in_memory().unwrap();
    let task_service = TaskService::new(&db);
    let task = task_service
        .create_task("Web task", None, None, None)
        .unwrap();
    db.set_current_task_id(task.id).unwrap();

    let app = test_router(db);

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/todo")
                .header("content-type", "application/x-www-form-urlencoded")
                .body(Body::from("content=%20%20%20"))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn add_scrap_rejects_empty_content() {
    let db = Database::new_in_memory().unwrap();
    let task_service = TaskService::new(&db);
    let task = task_service
        .create_task("Web task", None, None, None)
        .unwrap();
    db.set_current_task_id(task.id).unwrap();

    let app = test_router(db);

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/scrap")
                .header("content-type", "application/x-www-form-urlencoded")
                .body(Body::from("content=%20%20"))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn update_todo_rejects_reopen_from_done() {
    let db = Database::new_in_memory().unwrap();
    let task_service = TaskService::new(&db);
    let task = task_service
        .create_task("Web task", None, None, None)
        .unwrap();
    db.set_current_task_id(task.id).unwrap();

    let todo_service = TodoService::new(&db);
    let todo = todo_service.add_todo(task.id, "Done item", false).unwrap();
    todo_service
        .update_status(todo.id, TodoStatus::Done)
        .unwrap();

    let app = test_router(db);

    let response = app
        .oneshot(
            Request::builder()
                .method("PATCH")
                .uri("/api/todo/1/pending")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    let body = http_body_util::BodyExt::collect(response.into_body())
        .await
        .unwrap()
        .to_bytes();
    let text = String::from_utf8(body.to_vec()).unwrap();
    assert!(text.contains("reopen is not allowed"));
}

#[tokio::test]
async fn index_renders_without_active_task() {
    let db = Database::new_in_memory().unwrap();
    let app = test_router(db);

    let response = app
        .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = http_body_util::BodyExt::collect(response.into_body())
        .await
        .unwrap()
        .to_bytes();
    let html = String::from_utf8(body.to_vec()).unwrap();
    assert!(html.contains("<html") || html.contains("track"));
    assert!(html.contains("htmx.org@2.0.10"));
    assert!(html.contains("htmx-ext-sse@2.2.4"));
    assert!(html.contains("sse-connect=\"/api/sse\""));
    assert!(html.contains("hx-ext=\"sse\""));
}

#[tokio::test]
async fn index_with_task_uses_named_sse_triggers() {
    let db = Database::new_in_memory().unwrap();
    let task_service = TaskService::new(&db);
    let task = task_service
        .create_task("Web task", None, None, None)
        .unwrap();
    db.set_current_task_id(task.id).unwrap();

    let app = test_router(db);
    let response = app
        .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = http_body_util::BodyExt::collect(response.into_body())
        .await
        .unwrap()
        .to_bytes();
    let html = String::from_utf8(body.to_vec()).unwrap();
    assert!(html.contains("hx-trigger=\"sse:todos, sse:worktrees\""));
    assert!(html.contains("hx-trigger=\"sse:todos, sse:worktrees, sse:repos\""));
    assert!(html.contains("hx-trigger=\"sse:ticket\""));
    assert!(html.contains("id=\"task-identity\""));
    assert!(html.contains("hx-trigger=\"sse:scraps\""));
    assert!(html.contains("hx-target=\"#todos-section\""));
    assert!(html.contains("hx-disinherit=\"hx-trigger\""));
}

#[tokio::test]
async fn add_todo_returns_todo_list_partial() {
    let db = Database::new_in_memory().unwrap();
    let task_service = TaskService::new(&db);
    let task = task_service
        .create_task("Web task", None, None, None)
        .unwrap();
    db.set_current_task_id(task.id).unwrap();

    let app = test_router(db);
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/todo")
                .header("content-type", "application/x-www-form-urlencoded")
                .body(Body::from("content=From+htmx"))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = http_body_util::BodyExt::collect(response.into_body())
        .await
        .unwrap()
        .to_bytes();
    let html = String::from_utf8(body.to_vec()).unwrap();
    assert!(html.contains("id=\"todos-section\""));
    assert!(html.contains("From htmx"));
}

#[tokio::test]
async fn add_plan_todo_marks_item_as_plan() {
    let db = Database::new_in_memory().unwrap();
    let task_service = TaskService::new(&db);
    let task = task_service
        .create_task("Web task", None, None, None)
        .unwrap();
    db.set_current_task_id(task.id).unwrap();

    let app = test_router(db);
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/todo")
                .header("content-type", "application/x-www-form-urlencoded")
                .body(Body::from("content=Weekly+notes&no_workspace=true"))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = http_body_util::BodyExt::collect(response.into_body())
        .await
        .unwrap()
        .to_bytes();
    let html = String::from_utf8(body.to_vec()).unwrap();
    assert!(html.contains("Weekly notes"));
    assert!(html.contains("todo-kind"));
    assert!(html.contains(">plan</span>"));
}

#[tokio::test]
async fn identity_partial_renders_task_name() {
    let db = Database::new_in_memory().unwrap();
    let task_service = TaskService::new(&db);
    let task = task_service
        .create_task("Web task", None, None, None)
        .unwrap();
    db.set_current_task_id(task.id).unwrap();

    let app = test_router(db);
    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/card/identity")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = http_body_util::BodyExt::collect(response.into_body())
        .await
        .unwrap()
        .to_bytes();
    let html = String::from_utf8(body.to_vec()).unwrap();
    assert!(html.contains("id=\"task-identity\""));
    assert!(html.contains("Web task"));
    assert!(!html.contains("hx-swap-oob"));
}

#[tokio::test]
async fn update_ticket_returns_identity_out_of_band() {
    let db = Database::new_in_memory().unwrap();
    let task_service = TaskService::new(&db);
    let task = task_service
        .create_task("Web task", None, None, None)
        .unwrap();
    db.set_current_task_id(task.id).unwrap();

    let app = test_router(db);
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/ticket")
                .header("content-type", "application/x-www-form-urlencoded")
                .body(Body::from("ticket_id=NOTES-12"))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = http_body_util::BodyExt::collect(response.into_body())
        .await
        .unwrap()
        .to_bytes();
    let html = String::from_utf8(body.to_vec()).unwrap();
    assert!(html.contains("id=\"ticket-section\""));
    assert!(html.contains("NOTES-12"));
    assert!(html.contains("id=\"task-identity\""));
    assert!(html.contains("hx-swap-oob=\"outerHTML\""));
    assert!(html.contains("ticket-badge"));
}

#[tokio::test]
async fn workflow_partial_renders_for_active_task() {
    let db = Database::new_in_memory().unwrap();
    let task_service = TaskService::new(&db);
    let task = task_service
        .create_task("Web task", None, None, None)
        .unwrap();
    db.set_current_task_id(task.id).unwrap();

    let app = test_router(db);
    let response = app
        .oneshot(
            Request::builder()
                .uri("/partials/workflow")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = http_body_util::BodyExt::collect(response.into_body())
        .await
        .unwrap()
        .to_bytes();
    let html = String::from_utf8(body.to_vec()).unwrap();
    assert!(html.contains("id=\"workflow-section\""));
    assert!(html.contains("track todo add"));
    assert!(!html.contains("track repo add"));
}

#[tokio::test]
async fn workflow_partial_for_plan_todo_is_execute_not_repo_setup() {
    let db = Database::new_in_memory().unwrap();
    let task_service = TaskService::new(&db);
    let task = task_service
        .create_task("Notes task", None, None, None)
        .unwrap();
    db.set_current_task_id(task.id).unwrap();

    let todo_service = TodoService::new(&db);
    todo_service
        .add_todo(
            task.id,
            "Weekly notes",
            track::models::TodoAddOptions::from_flags(false, true),
        )
        .unwrap();

    let app = test_router(db);
    let response = app
        .oneshot(
            Request::builder()
                .uri("/partials/workflow")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = http_body_util::BodyExt::collect(response.into_body())
        .await
        .unwrap()
        .to_bytes();
    let html = String::from_utf8(body.to_vec()).unwrap();
    assert!(html.contains("phase-execute"));
    assert!(html.contains("track todo done"));
    assert!(!html.contains("track repo add"));
}
