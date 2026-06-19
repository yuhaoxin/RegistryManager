use thiserror::Error;

#[derive(Debug, Error)]
pub enum DockerError {
    #[error("Docker 守护进程不可用：{0}")]
    DockerUnavailable(String),
    #[error("不支持远程 Docker 上下文：{0}")]
    RemoteContext(String),
    #[error("检查容器 {container_id} 失败：{source}")]
    InspectFailed {
        container_id: String,
        source: bollard::errors::Error,
    },
    #[error("未找到容器：{0}")]
    NotFound(String),
}
