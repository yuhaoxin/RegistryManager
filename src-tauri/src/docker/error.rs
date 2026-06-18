use thiserror::Error;

#[derive(Debug, Error)]
pub enum DockerError {
    #[error("Docker daemon is unavailable: {0}")]
    DockerUnavailable(String),
    #[error("remote Docker contexts are not supported: {0}")]
    RemoteContext(String),
    #[error("failed to inspect container {container_id}: {source}")]
    InspectFailed {
        container_id: String,
        source: bollard::errors::Error,
    },
    #[error("container not found: {0}")]
    NotFound(String),
}
