use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use serde::Serialize;
use sqlx::SqlitePool;
use tokio::task::AbortHandle;

use crate::credentials::CredentialError;
use crate::docker::DockerError;
use crate::registry::RegistryError;
use crate::store::{connect_app_database, StoreError};

pub mod audit;
pub mod cache;
pub mod delete;
pub mod docker;
pub mod gc;
pub mod registry;

#[derive(Clone)]
pub struct AppState {
    pub pool: SqlitePool,
    pub refresh_tasks: Arc<Mutex<HashMap<String, AbortHandle>>>,
    pub gc_locks: Arc<Mutex<HashMap<String, Arc<tokio::sync::Mutex<()>>>>>,
}

impl AppState {
    pub async fn new() -> Result<Self, StoreError> {
        Ok(Self {
            pool: connect_app_database().await?,
            refresh_tasks: Arc::new(Mutex::new(HashMap::new())),
            gc_locks: Arc::new(Mutex::new(HashMap::new())),
        })
    }

    #[cfg(test)]
    pub async fn in_memory() -> Result<Self, StoreError> {
        Ok(Self {
            pool: crate::store::connect_database(std::path::Path::new(":memory:")).await?,
            refresh_tasks: Arc::new(Mutex::new(HashMap::new())),
            gc_locks: Arc::new(Mutex::new(HashMap::new())),
        })
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppError {
    pub code: String,
    pub message: String,
    pub details: Option<String>,
}

impl AppError {
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            details: None,
        }
    }

    pub fn with_details(
        code: impl Into<String>,
        message: impl Into<String>,
        details: impl Into<String>,
    ) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            details: Some(details.into()),
        }
    }
}

impl From<DockerError> for AppError {
    fn from(value: DockerError) -> Self {
        match value {
            DockerError::DockerUnavailable(message) => Self::with_details(
                "docker_unavailable",
                "Docker daemon is unavailable.",
                message,
            ),
            DockerError::RemoteContext(context) => Self::with_details(
                "remote_docker_context",
                "Remote Docker contexts are not supported.",
                context,
            ),
            DockerError::InspectFailed {
                container_id,
                source,
            } => Self::with_details(
                "docker_inspect_failed",
                "Failed to inspect registry container.",
                format!("{container_id}: {source}"),
            ),
            DockerError::NotFound(message) => {
                Self::with_details("container_not_found", "Container was not found.", message)
            }
        }
    }
}

impl From<RegistryError> for AppError {
    fn from(value: RegistryError) -> Self {
        match value {
            RegistryError::RequestFailed(error) => Self::with_details(
                "registry_unreachable",
                "Registry API is unreachable.",
                error.to_string(),
            ),
            RegistryError::UnexpectedStatus(status) => Self::with_details(
                "registry_api_error",
                "Registry API returned an unexpected status.",
                status.to_string(),
            ),
            RegistryError::Unauthorized => Self::new(
                "registry_unauthorized",
                "Registry authentication is required before this manifest can be deleted.",
            ),
            RegistryError::Forbidden => Self::new(
                "registry_forbidden",
                "Registry denied permission to delete this manifest.",
            ),
            RegistryError::InvalidUrl => Self::new("invalid_registry_url", "Invalid registry URL."),
            RegistryError::UnsupportedMediaType(media_type) => Self::with_details(
                "unsupported_manifest_media_type",
                "Manifest media type is not supported.",
                media_type,
            ),
            RegistryError::DigestNotFound => Self::new(
                "manifest_digest_missing",
                "Registry did not return Docker-Content-Digest.",
            ),
            RegistryError::JsonParse(error) => Self::with_details(
                "registry_json_parse_failed",
                "Registry response could not be parsed.",
                error.to_string(),
            ),
            RegistryError::NotFound => Self::new(
                "registry_resource_not_found",
                "Registry resource was not found.",
            ),
        }
    }
}

impl From<StoreError> for AppError {
    fn from(value: StoreError) -> Self {
        Self::with_details(
            "store_error",
            "Local cache operation failed.",
            value.to_string(),
        )
    }
}

impl From<CredentialError> for AppError {
    fn from(value: CredentialError) -> Self {
        Self::with_details(
            "credential_store_error",
            "Registry credentials could not be updated or loaded.",
            value.to_string(),
        )
    }
}

impl From<uuid::Error> for AppError {
    fn from(value: uuid::Error) -> Self {
        Self::with_details(
            "invalid_profile_id",
            "Invalid registry profile id.",
            value.to_string(),
        )
    }
}
