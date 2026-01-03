//! SSE (Server-Sent Events) handler for real-time updates.

use crate::webui::routes::WebState;
use axum::response::sse::{Event, KeepAlive, Sse};
use futures::stream::Stream;
use std::convert::Infallible;
use std::time::Duration;
use tokio_stream::wrappers::BroadcastStream;
use tokio_stream::StreamExt;

/// SSE endpoint handler
pub async fn sse_handler(
    axum::extract::State(state): axum::extract::State<WebState>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let rx = state.app.sse_tx.subscribe();

    let stream = BroadcastStream::new(rx).filter_map(|result| {
        match result {
            Ok(event) => {
                let data = serde_json::to_string(&event).unwrap_or_default();
                Some(Ok(Event::default().event("update").data(data)))
            }
            Err(_) => None, // Ignore lagged messages
        }
    });

    Sse::new(stream).keep_alive(
        KeepAlive::new()
            .interval(Duration::from_secs(30))
            .text("keep-alive"),
    )
}
