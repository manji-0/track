//! # WebUI Module
//!
//! Provides a web-based user interface for track.
//! Run `track webui` to start the web server.
//!
//! ## Features
//!
//! - Modern, rich UI displaying task status
//! - Add/delete TODOs and scraps through the web interface
//! - Real-time updates via Server-Sent Events (SSE)

mod error;
mod routes;
mod server;
mod sse;
mod state;
mod templates;

pub use routes::WebState;
pub use server::{build_router, start_server};
pub use state::AppState;
pub use templates::Templates;
