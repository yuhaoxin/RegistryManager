use thiserror::Error;

#[derive(Debug, Error)]
pub enum StoreError {
    #[error("应用数据目录不可用")]
    AppDataDirUnavailable,
    #[error("创建数据库目录失败：{0}")]
    DirectoryCreate(#[from] std::io::Error),
    #[error("数据库错误：{0}")]
    Database(#[from] sqlx::Error),
    #[error("解析时间戳失败：{0}")]
    TimestampParse(#[from] chrono::ParseError),
    #[error("解析 UUID 失败：{0}")]
    UuidParse(#[from] uuid::Error),
}
