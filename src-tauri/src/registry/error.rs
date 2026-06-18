use thiserror::Error;

#[derive(Debug, Error)]
pub enum RegistryError {
    #[error("registry request failed: {0}")]
    RequestFailed(#[from] reqwest::Error),
    #[error("unexpected registry status: {0}")]
    UnexpectedStatus(u16),
    #[error("registry authentication required")]
    Unauthorized,
    #[error("registry operation forbidden")]
    Forbidden,
    #[error("invalid registry url")]
    InvalidUrl,
    #[error("unsupported manifest media type: {0}")]
    UnsupportedMediaType(String),
    #[error("Docker-Content-Digest header not found")]
    DigestNotFound,
    #[error("failed to parse registry JSON: {0}")]
    JsonParse(#[from] serde_json::Error),
    #[error("registry resource not found")]
    NotFound,
}
