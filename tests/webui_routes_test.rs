use axum::body::Body;
use axum::http::{Request, StatusCode};
use std::sync::Arc;
use tower::ServiceExt;
use track::db::Database;
use track::services::{TaskService, TodoService};
use track::webui::{build_router, AppState, Templates, WebState};

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
    assert_eq!(json["vcs_mode"], "jj");
    assert!(json["guardrails"]["reopen_forbidden"].as_bool().unwrap());
    assert!(json["guardrails"]["must_use_jj_skill"].as_bool().unwrap());
    assert_eq!(json["jj"]["skill"], "jj");
    assert!(json.get("git").is_none());
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
    todo_service.update_status(todo.id, "done").unwrap();

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
    assert!(text.contains("cannot be reopened"));
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
}
