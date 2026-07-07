//! HTTP error mapping for the WebUI.

use crate::utils::TrackError;
use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};

/// WebUI error with an appropriate HTTP status code.
pub struct WebError {
    status: StatusCode,
    message: String,
}

impl WebError {
    fn status_for(err: &TrackError) -> StatusCode {
        match err {
            TrackError::NoActiveTask
            | TrackError::EmptyTaskName
            | TrackError::EmptyTodoContent
            | TrackError::EmptyScrapContent
            | TrackError::InvalidTicketFormat(_)
            | TrackError::InvalidStatus(_)
            | TrackError::InvalidStatusTransition { .. }
            | TrackError::TodoReopenForbidden { .. }
            | TrackError::TodoCompleteRequiresDoneCommand
            | TrackError::InvalidUrl(_)
            | TrackError::DuplicateTicket(_, _)
            | TrackError::TaskArchived(_)
            | TrackError::NoRepositoriesRegistered
            | TrackError::RepoHasPendingChanges(_)
            | TrackError::WorktreeFlagRemoved
            | TrackError::SyncUseJjTask { .. }
            | TrackError::JjTaskNotCompleted { .. }
            | TrackError::CurrentDirectoryNotRegistered
            | TrackError::WorkspaceHasUncommittedChanges { .. }
            | TrackError::BookmarkNotFound { .. }
            | TrackError::NoWorkspacePathsAvailable
            | TrackError::TodoIndexNotFound(_)
            | TrackError::TodoNotPending(_)
            | TrackError::NoPendingTodos
            | TrackError::LinkNotFound(_) => StatusCode::BAD_REQUEST,
            TrackError::TaskNotFound(_)
            | TrackError::TodoNotFound(_)
            | TrackError::WorktreeNotFound(_) => StatusCode::NOT_FOUND,
            TrackError::Database(_)
            | TrackError::UncommittedWorkspaces(_)
            | TrackError::TodoCompletionDbFailed { .. }
            | TrackError::Jj(_)
            | TrackError::NotJjRepository(_)
            | TrackError::BookmarkExists(_)
            | TrackError::FailedRepoStatusCheck(_)
            | TrackError::WorkspaceRemovalFailed(_)
            | TrackError::WorkspaceStatusCheckFailed { .. }
            | TrackError::SerializationFailed(_)
            | TrackError::JjTaskMapInvalid { .. }
            | TrackError::PathResolutionFailed(_)
            | TrackError::Io(_)
            | TrackError::Cancelled
            | TrackError::Other(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn from_track(err: TrackError) -> Self {
        let status = Self::status_for(&err);
        Self {
            status,
            message: err.to_string(),
        }
    }
}

impl<E> From<E> for WebError
where
    E: Into<anyhow::Error>,
{
    fn from(err: E) -> Self {
        match err.into().downcast::<TrackError>() {
            Ok(track) => Self::from_track(track),
            Err(err) => Self {
                status: StatusCode::INTERNAL_SERVER_ERROR,
                message: err.to_string(),
            },
        }
    }
}

impl IntoResponse for WebError {
    fn into_response(self) -> Response {
        (self.status, format!("Error: {}", self.message)).into_response()
    }
}
