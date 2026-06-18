use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RegistryProfile {
    pub id: Uuid,
    pub container_id: String,
    pub container_name: String,
    pub image: String,
    pub registry_url: String,
    pub port_mapping: String,
    pub config_path: Option<String>,
    pub storage_mounts: String,
    pub selected_at: DateTime<Utc>,
    pub last_health_check_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RepositoryCache {
    pub registry_id: Uuid,
    pub repository_name: String,
    pub tag_count: i64,
    pub last_synced_at: Option<DateTime<Utc>>,
    pub sync_status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ManifestCache {
    pub registry_id: Uuid,
    pub repository_name: String,
    pub tag: String,
    pub digest: String,
    pub media_type: String,
    pub platform_summary: Option<String>,
    pub raw_json: String,
    pub last_synced_at: DateTime<Utc>,
    pub gc_status: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AuditEvent {
    pub id: Uuid,
    pub timestamp: DateTime<Utc>,
    pub action: String,
    pub registry_id: Option<Uuid>,
    pub container_id: Option<String>,
    pub repository_name: Option<String>,
    pub tag: Option<String>,
    pub digest: Option<String>,
    pub status: String,
    pub duration_ms: Option<i64>,
    pub error_message: Option<String>,
    pub log_excerpt: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct GcTransaction {
    pub id: Uuid,
    pub registry_id: Uuid,
    pub container_id: String,
    pub original_state: Option<String>,
    pub original_image: String,
    pub mount_summary: String,
    pub config_path: Option<String>,
    pub started_at: DateTime<Utc>,
    pub finished_at: Option<DateTime<Utc>>,
    pub exit_code: Option<i64>,
    pub log_path: Option<String>,
    pub recovery_action: Option<String>,
    pub final_health_status: Option<String>,
}
