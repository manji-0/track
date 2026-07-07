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
            | TrackError::InvalidAlias(_)
            | TrackError::AliasInUse { .. }
            | TrackError::RepoAlreadyRegistered
            | TrackError::TaskRepoIndexNotFound(_)
            | TrackError::LinkIndexNotFound(_)
            | TrackError::TaskReferenceNotFound(_)
            | TrackError::LinkNotFound(_) => StatusCode::BAD_REQUEST,
            TrackError::TaskNotFound(_)
            | TrackError::TodoNotFound(_)
            | TrackError::WorktreeNotFound(_)
            | TrackError::TaskRepoNotFound(_) => StatusCode::NOT_FOUND,
            TrackError::Database(_)
            | TrackError::UncommittedWorkspaces(_)
            | TrackError::TodoCompletionDbFailed { .. }
            | TrackError::Jj(_)
            | TrackError::Git(_)
            | TrackError::NotJjRepository(_)
            | TrackError::NotGitRepository(_)
            | TrackError::BookmarkExists(_)
            | TrackError::FailedRepoStatusCheck(_)
            | TrackError::WorkspaceRemovalFailed(_)
            | TrackError::WorkspaceStatusCheckFailed { .. }
            | TrackError::SerializationFailed(_)
            | TrackError::JjTaskMapInvalid { .. }
            | TrackError::PathResolutionFailed(_)
            | TrackError::Io(_)
            | TrackError::Cancelled
            | TrackError::TemplateRenderFailed { .. }
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

impl From<TrackError> for WebError {
    fn from(err: TrackError) -> Self {
        Self::from_track(err)
    }
}

impl From<serde_json::Error> for WebError {
    fn from(err: serde_json::Error) -> Self {
        Self::from_track(TrackError::SerializationFailed(err.to_string()))
    }
}

impl From<anyhow::Error> for WebError {
    fn from(err: anyhow::Error) -> Self {
        match err.downcast::<TrackError>() {
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
