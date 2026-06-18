use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RegistryContainerSummary {
    pub id: String,
    pub name: String,
    pub image: String,
    pub registry_url: Option<String>,
    pub ports: Vec<ContainerPort>,
    pub mounts: Vec<ContainerMount>,
    pub state: Option<String>,
    pub env: Vec<String>,
    pub restart_policy: Option<String>,
    pub health_status: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ContainerPort {
    pub container_port: u16,
    pub host_ip: Option<String>,
    pub host_port: Option<u16>,
    pub protocol: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ContainerMount {
    pub source: Option<String>,
    pub destination: Option<String>,
    pub mode: Option<String>,
    pub mount_type: Option<String>,
}
