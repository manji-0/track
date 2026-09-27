//! SSE (Server-Sent Events) handler for real-time updates.

use crate::models::TaskId;
use crate::webui::routes::WebState;
use crate::webui::state::SseSubscription;
use axum::extract::{Query, State};
use axum::response::sse::{Event, KeepAlive, Sse};
use futures::stream::Stream;
use serde::Deserialize;
use std::convert::Infallible;
use std::time::Duration;
use tokio_stream::StreamExt;
use tokio_stream::wrappers::BroadcastStream;

/// Tab scope sent by the page: `/api/sse?task=<id>&follow=true`.
#[derive(Deserialize)]
pub struct SseQuery {
    task: Option<i64>,
    #[serde(default)]
    follow: bool,
}

/// SSE endpoint handler
pub async fn sse_handler(
    State(state): State<WebState>,
    Query(query): Query<SseQuery>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let subscription = SseSubscription {
        task_id: query.task.map(TaskId::from_i64),
        follow_current: query.follow,
    };
    let rx = state.app.sse_tx.subscribe();

    let stream = BroadcastStream::new(rx).filter_map(move |result| {
        match result {
            Ok(message) if subscription.accepts(&message) => {
                let name = message.event.event_name();
                Some(Ok(Event::default().event(name).data(name)))
            }
            _ => None, // Other tabs' events, or lagged messages
        }
    });

    Sse::new(stream).keep_alive(
        KeepAlive::new()
            .interval(Duration::from_secs(30))
            .text("keep-alive"),
    )
}
