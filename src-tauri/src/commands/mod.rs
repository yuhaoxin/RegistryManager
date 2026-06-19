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
            DockerError::DockerUnavailable(message) => {
                Self::with_details("docker_unavailable", "Docker 守护进程不可用。", message)
            }
            DockerError::RemoteContext(context) => Self::with_details(
                "remote_docker_context",
                "不支持远程 Docker 上下文。",
                context,
            ),
            DockerError::InspectFailed {
                container_id,
                source,
            } => Self::with_details(
                "docker_inspect_failed",
                "检查 Registry 容器失败。",
                format!("{container_id}: {source}"),
            ),
            DockerError::NotFound(message) => {
                Self::with_details("container_not_found", "未找到容器。", message)
            }
        }
    }
}

impl From<RegistryError> for AppError {
    fn from(value: RegistryError) -> Self {
        match value {
            RegistryError::RequestFailed(error) => Self::with_details(
                "registry_unreachable",
                "无法连接 Registry API。",
                error.to_string(),
            ),
            RegistryError::UnexpectedStatus(status) => Self::with_details(
                "registry_api_error",
                "Registry API 返回了非预期状态。",
                status.to_string(),
            ),
            RegistryError::Unauthorized => Self::new(
                "registry_unauthorized",
                "删除此清单前需要完成 Registry 身份验证。",
            ),
            RegistryError::Forbidden => {
                Self::new("registry_forbidden", "Registry 拒绝删除此清单的权限。")
            }
            RegistryError::InvalidUrl => Self::new("invalid_registry_url", "Registry URL 无效。"),
            RegistryError::UnsupportedMediaType(media_type) => Self::with_details(
                "unsupported_manifest_media_type",
                "不支持此清单媒体类型。",
                media_type,
            ),
            RegistryError::DigestNotFound => Self::new(
                "manifest_digest_missing",
                "Registry 未返回 Docker-Content-Digest。",
            ),
            RegistryError::JsonParse(error) => Self::with_details(
                "registry_json_parse_failed",
                "无法解析 Registry 响应。",
                error.to_string(),
            ),
            RegistryError::NotFound => {
                Self::new("registry_resource_not_found", "未找到 Registry 资源。")
            }
        }
    }
}

impl From<StoreError> for AppError {
    fn from(value: StoreError) -> Self {
        Self::with_details("store_error", "本地缓存操作失败。", value.to_string())
    }
}

impl From<CredentialError> for AppError {
    fn from(value: CredentialError) -> Self {
        Self::with_details(
            "credential_store_error",
            "无法更新或加载 Registry 凭据。",
            value.to_string(),
        )
    }
}

impl From<uuid::Error> for AppError {
    fn from(value: uuid::Error) -> Self {
        Self::with_details(
            "invalid_profile_id",
            "Registry 配置 ID 无效。",
            value.to_string(),
        )
    }
}
