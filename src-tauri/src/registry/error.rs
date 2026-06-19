use thiserror::Error;

#[derive(Debug, Error)]
pub enum RegistryError {
    #[error("Registry 请求失败：{0}")]
    RequestFailed(#[from] reqwest::Error),
    #[error("非预期 Registry 状态：{0}")]
    UnexpectedStatus(u16),
    #[error("需要 Registry 身份验证")]
    Unauthorized,
    #[error("Registry 操作被禁止")]
    Forbidden,
    #[error("Registry URL 无效")]
    InvalidUrl,
    #[error("不支持的清单媒体类型：{0}")]
    UnsupportedMediaType(String),
    #[error("未找到 Docker-Content-Digest 头")]
    DigestNotFound,
    #[error("解析 Registry JSON 失败：{0}")]
    JsonParse(#[from] serde_json::Error),
    #[error("未找到 Registry 资源")]
    NotFound,
}
