use thiserror::Error;

#[derive(Debug, Error)]
pub enum StoreError {
    #[error("application data directory is unavailable")]
    AppDataDirUnavailable,
    #[error("failed to create database directory: {0}")]
    DirectoryCreate(#[from] std::io::Error),
    #[error("database error: {0}")]
    Database(#[from] sqlx::Error),
    #[error("failed to parse timestamp: {0}")]
    TimestampParse(#[from] chrono::ParseError),
    #[error("failed to parse uuid: {0}")]
    UuidParse(#[from] uuid::Error),
}
