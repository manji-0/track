//! WebUI server implementation.

use crate::webui::routes::{self, WebState};
use crate::webui::sse::sse_handler;
use crate::webui::state::AppState;
use crate::webui::templates::Templates;
use axum::{
    routing::{delete, get, post},
    Router,
};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpListener;
use tower_http::services::ServeDir;

/// Start the WebUI server
pub async fn start_server(port: u16, open_browser: bool) -> anyhow::Result<()> {
    // Initialize application state
    let app_state = AppState::new()?;
    
    // Initialize templates (embedded for single-binary distribution)
    let templates = Arc::new(Templates::embedded());
    
    let web_state = WebState {
        app: app_state,
        templates,
    };
    
    // Build router
    let app = Router::new()
        // Pages
        .route("/", get(routes::index))
        // API endpoints
        .route("/api/status", get(routes::api_status))
        .route("/api/todo", post(routes::add_todo))
        .route("/api/todo/:id", delete(routes::delete_todo))
        .route("/api/scrap", post(routes::add_scrap))
        .route("/api/description", post(routes::update_description))
        // SSE endpoint
        .route("/api/sse", get(sse_handler))
        // Static files (CSS, JS)
        .nest_service("/static", ServeDir::new("static"))
        .with_state(web_state);
    
    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    
    println!("Starting track webui server...");
    println!("  → http://localhost:{}", port);
    println!();
    println!("Press Ctrl+C to stop the server.");
    
    // Open browser if requested
    if open_browser {
        let url = format!("http://localhost:{}", port);
        if let Err(e) = open::that(&url) {
            eprintln!("Warning: Failed to open browser: {}", e);
        }
    }
    
    // Start server
    let listener = TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    
    Ok(())
}
