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

mod routes;
mod server;
mod sse;
mod state;
mod templates;

pub use server::start_server;
