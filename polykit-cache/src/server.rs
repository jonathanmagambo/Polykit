//! HTTP server for artifact cache.

use std::sync::Arc;

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, head, put},
    Router,
};
use tower_http::trace::TraceLayer;

use crate::storage::Storage;
use crate::verification::Verifier;

/// Server state shared across handlers.
#[derive(Clone)]
pub struct AppState {
    storage: Arc<Storage>,
    verifier: Arc<Verifier>,
}

impl AppState {
    /// Creates new app state.
    pub fn new(storage: Storage, verifier: Verifier) -> Self {
        Self {
            storage: Arc::new(storage),
            verifier: Arc::new(verifier),
        }
    }
}

/// Creates the HTTP router.
pub fn create_router(state: AppState) -> Router {
    Router::new()
        .route("/v1/artifacts/:cache_key", put(upload_artifact))
        .route("/v1/artifacts/:cache_key", get(download_artifact))
        .route("/v1/artifacts/:cache_key", head(check_artifact))
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}

/// Uploads an artifact.
///
/// PUT /v1/artifacts/{cache_key}
async fn upload_artifact(
    State(state): State<AppState>,
    Path(cache_key): Path<String>,
    body: axum::body::Body,
) -> Result<Response, ServerError> {
    // Validate cache key format
    if !cache_key.chars().all(|c| c.is_ascii_hexdigit()) || cache_key.len() < 32 {
        return Err(ServerError::BadRequest(format!(
            "Invalid cache key format: {}",
            cache_key
        )));
    }

    // Stream body to bytes with size limit
    let max_size = state.storage.max_artifact_size() as usize;
    let bytes = axum::body::to_bytes(body, max_size)
        .await
        .map_err(|e| {
            if e.to_string().contains("too large") {
                ServerError::PayloadTooLarge(format!(
                    "Artifact size exceeds maximum {}",
                    max_size
                ))
            } else {
                ServerError::Internal(format!("Failed to read request body: {}", e))
            }
        })?;

    // Verify artifact
    let (artifact, hash) = state
        .verifier
        .verify_upload(&bytes, &cache_key)
        .map_err(|e| ServerError::UnprocessableEntity(e.to_string()))?;

    // Store artifact
    state
        .storage
        .store_artifact(&cache_key, bytes.to_vec(), hash, &artifact)
        .await
        .map_err(|e| {
            if e.to_string().contains("already exists") {
                ServerError::Conflict(format!("Artifact {} already exists", cache_key))
            } else {
                ServerError::Internal(format!("Failed to store artifact: {}", e))
            }
        })?;

    Ok(StatusCode::CREATED.into_response())
}

/// Downloads an artifact.
///
/// GET /v1/artifacts/{cache_key}
async fn download_artifact(
    State(state): State<AppState>,
    Path(cache_key): Path<String>,
) -> Result<Response, ServerError> {
    // Validate cache key format
    if !cache_key.chars().all(|c| c.is_ascii_hexdigit()) || cache_key.len() < 32 {
        return Err(ServerError::BadRequest(format!(
            "Invalid cache key format: {}",
            cache_key
        )));
    }

    // Check if artifact exists
    if !state.storage.has_artifact(&cache_key) {
        return Err(ServerError::NotFound);
    }

    // Read artifact
    let data = state
        .storage
        .read_artifact(&cache_key)
        .await
        .map_err(|e| ServerError::Internal(format!("Failed to read artifact: {}", e)))?;

    // Read metadata for headers
    let metadata = state
        .storage
        .read_metadata(&cache_key)
        .await
        .map_err(|e| ServerError::Internal(format!("Failed to read metadata: {}", e)))?;

    // Create response with proper headers
    let response = Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "application/zstd")
        .header("Content-Length", data.len())
        .header("X-Artifact-Hash", &metadata.hash)
        .body(axum::body::Body::from(data))
        .map_err(|e| ServerError::Internal(format!("Failed to create response: {}", e)))?;

    Ok(response)
}

/// Checks if an artifact exists.
///
/// HEAD /v1/artifacts/{cache_key}
async fn check_artifact(
    State(state): State<AppState>,
    Path(cache_key): Path<String>,
) -> Result<Response, ServerError> {
    // Validate cache key format
    if !cache_key.chars().all(|c| c.is_ascii_hexdigit()) || cache_key.len() < 32 {
        return Err(ServerError::BadRequest(format!(
            "Invalid cache key format: {}",
            cache_key
        )));
    }

    if state.storage.has_artifact(&cache_key) {
        // Read metadata for headers
        if let Ok(metadata) = state.storage.read_metadata(&cache_key).await {
            let response = Response::builder()
                .status(StatusCode::OK)
                .header("Content-Type", "application/zstd")
                .header("Content-Length", metadata.size)
                .header("X-Artifact-Hash", &metadata.hash)
                .body(axum::body::Body::empty())
                .map_err(|e| ServerError::Internal(format!("Failed to create response: {}", e)))?;

            return Ok(response);
        }
    }

    Err(ServerError::NotFound)
}

/// Server error types.
#[derive(Debug)]
pub enum ServerError {
    BadRequest(String),
    Conflict(String),
    PayloadTooLarge(String),
    UnprocessableEntity(String),
    NotFound,
    Internal(String),
}

impl IntoResponse for ServerError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            ServerError::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg),
            ServerError::Conflict(msg) => (StatusCode::CONFLICT, msg),
            ServerError::PayloadTooLarge(msg) => (StatusCode::PAYLOAD_TOO_LARGE, msg),
            ServerError::UnprocessableEntity(msg) => (StatusCode::UNPROCESSABLE_ENTITY, msg),
            ServerError::NotFound => (StatusCode::NOT_FOUND, "Artifact not found".to_string()),
            ServerError::Internal(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg),
        };

        let body = axum::Json(serde_json::json!({ "error": message }));
        (status, body).into_response()
    }
}
