use serde::Serialize;

use crate::docker::{
    discover_registry_containers as discover, DockerClient, RegistryContainerSummary,
};

use super::AppError;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DockerStatus {
    pub reachable: bool,
    pub version: Option<String>,
    pub context: String,
    pub error: Option<String>,
}

#[tauri::command]
pub async fn get_docker_status() -> Result<DockerStatus, AppError> {
    let context = std::env::var("DOCKER_CONTEXT").unwrap_or_else(|_| "default".to_string());
    match DockerClient::connect_local().await {
        Ok(client) => {
            let version = client
                .docker()
                .version()
                .await
                .ok()
                .and_then(|version| version.version);
            Ok(DockerStatus {
                reachable: true,
                version,
                context,
                error: None,
            })
        }
        Err(error) => Ok(DockerStatus {
            reachable: false,
            version: None,
            context,
            error: Some(error.to_string()),
        }),
    }
}

#[tauri::command]
pub async fn discover_registry_containers() -> Result<Vec<RegistryContainerSummary>, AppError> {
    let client = DockerClient::connect_local().await?;
    Ok(discover(&client).await?)
}
