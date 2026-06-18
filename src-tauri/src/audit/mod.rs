use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{Row, SqlitePool};
use uuid::Uuid;

use crate::store::StoreError;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AuditAction {
    DeleteManifest,
    LocalGc,
}

impl AuditAction {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::DeleteManifest => "delete_manifest",
            Self::LocalGc => "local_gc",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AuditEvent {
    pub id: Uuid,
    pub timestamp: DateTime<Utc>,
    pub action: AuditAction,
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

pub async fn log_audit_event(pool: &SqlitePool, event: &AuditEvent) -> Result<(), StoreError> {
    sqlx::query(
        r#"
        INSERT INTO audit_events (
            id, timestamp, action, registry_id, container_id, repository_name, tag,
            digest, status, duration_ms, error_message, log_excerpt
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(event.id.to_string())
    .bind(event.timestamp.to_rfc3339())
    .bind(event.action.as_str())
    .bind(event.registry_id.map(|id| id.to_string()))
    .bind(&event.container_id)
    .bind(&event.repository_name)
    .bind(&event.tag)
    .bind(&event.digest)
    .bind(&event.status)
    .bind(event.duration_ms)
    .bind(&event.error_message)
    .bind(&event.log_excerpt)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn list_audit_events(
    pool: &SqlitePool,
    limit: i64,
    offset: i64,
) -> Result<Vec<AuditEvent>, StoreError> {
    let rows = sqlx::query("SELECT * FROM audit_events ORDER BY timestamp DESC LIMIT ? OFFSET ?")
        .bind(limit.clamp(1, 500))
        .bind(offset.max(0))
        .fetch_all(pool)
        .await?;

    rows.into_iter().map(row_to_audit_event).collect()
}

fn row_to_audit_event(row: sqlx::sqlite::SqliteRow) -> Result<AuditEvent, StoreError> {
    let timestamp: String = row.try_get("timestamp")?;
    let action: String = row.try_get("action")?;
    let registry_id: Option<String> = row.try_get("registry_id")?;

    Ok(AuditEvent {
        id: Uuid::parse_str(row.try_get::<String, _>("id")?.as_str())?,
        timestamp: DateTime::parse_from_rfc3339(&timestamp)?.with_timezone(&Utc),
        action: match action.as_str() {
            "local_gc" => AuditAction::LocalGc,
            _ => AuditAction::DeleteManifest,
        },
        registry_id: registry_id
            .map(|value| Uuid::parse_str(&value))
            .transpose()?,
        container_id: row.try_get("container_id")?,
        repository_name: row.try_get("repository_name")?,
        tag: row.try_get("tag")?,
        digest: row.try_get("digest")?,
        status: row.try_get("status")?,
        duration_ms: row.try_get("duration_ms")?,
        error_message: row.try_get("error_message")?,
        log_excerpt: row.try_get("log_excerpt")?,
    })
}
