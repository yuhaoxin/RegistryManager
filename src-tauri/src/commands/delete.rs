use std::collections::BTreeSet;
use std::time::Instant;

use chrono::Utc;
use serde::Serialize;
use tauri::State;
use uuid::Uuid;

use crate::audit::{log_audit_event, AuditAction, AuditEvent};
use crate::registry::{RegistryClient, RegistryError, DOCKER_MANIFEST_LIST, OCI_IMAGE_INDEX};
use crate::store::{
    get_registry_profile, list_manifest_cache, list_manifest_cache_by_digest,
    update_manifest_gc_status, RegistryProfile,
};

use super::{
    registry::ensure_local_registry_target, registry::registry_client_for_profile, AppError,
    AppState,
};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteImpact {
    pub repository: String,
    pub reference: String,
    pub digest: String,
    pub digest_suffix: String,
    pub media_type: String,
    pub affected_tags: Vec<String>,
    pub is_multi_arch: bool,
    pub warning: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteResult {
    pub digest: String,
    pub status: String,
    pub pending_gc: bool,
}

#[tauri::command]
pub async fn get_delete_impact(
    profile_id: String,
    repository: String,
    reference: String,
    state: State<'_, AppState>,
) -> Result<DeleteImpact, AppError> {
    let profile = require_profile(&state, &profile_id).await?;
    ensure_local_registry_target(&profile.registry_url).await?;
    let client = registry_client_for_profile(&profile)?;
    let digest = client.resolve_digest(&repository, &reference).await?;
    let (media_type, _) = client.fetch_manifest_raw(&repository, &digest).await?;
    let affected_tags = affected_tags(&state, &profile, &client, &repository, &digest).await?;
    let is_multi_arch = matches!(media_type.as_str(), DOCKER_MANIFEST_LIST | OCI_IMAGE_INDEX);

    Ok(DeleteImpact {
        repository,
        reference,
        digest_suffix: digest_suffix(&digest),
        digest,
        media_type,
        affected_tags,
        is_multi_arch,
        warning: "Storage may not be released until server-side GC completes.".to_string(),
    })
}

#[tauri::command]
pub async fn delete_manifest(
    profile_id: String,
    repository: String,
    reference: String,
    confirmed_digest_suffix: String,
    state: State<'_, AppState>,
) -> Result<DeleteResult, AppError> {
    let profile = require_profile(&state, &profile_id).await?;
    ensure_local_registry_target(&profile.registry_url).await?;
    let client = registry_client_for_profile(&profile)?;
    let started = Instant::now();
    let digest = client.resolve_digest(&repository, &reference).await?;
    let expected_suffix = digest_suffix(&digest);

    if confirmed_digest_suffix.trim() != expected_suffix {
        return Err(AppError::with_details(
            "delete_confirmation_mismatch",
            "Digest confirmation does not match the required suffix.",
            expected_suffix,
        ));
    }

    let result = client.delete_manifest(&repository, &digest).await;
    let (status, error_message) = match &result {
        Ok(()) => ("pending_gc".to_string(), None),
        Err(error) => ("failure".to_string(), Some(delete_error_message(error))),
    };

    log_audit_event(
        &state.pool,
        &AuditEvent {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            action: AuditAction::DeleteManifest,
            registry_id: Some(profile.id),
            container_id: Some(profile.container_id.clone()),
            repository_name: Some(repository.clone()),
            tag: Some(reference.clone()),
            digest: Some(digest.clone()),
            status: status.clone(),
            duration_ms: Some(started.elapsed().as_millis() as i64),
            error_message: error_message.clone(),
            log_excerpt: Some(
                serde_json::json!({
                    "registry": profile.registry_url,
                    "result": status,
                })
                .to_string(),
            ),
        },
    )
    .await?;

    if let Err(error) = result {
        return Err(AppError::with_details(
            delete_error_code(&error),
            delete_error_message(&error),
            error.to_string(),
        ));
    }

    update_manifest_gc_status(&state.pool, profile.id, &repository, &digest, "pending_gc").await?;

    Ok(DeleteResult {
        digest,
        status: "pending_gc".to_string(),
        pending_gc: true,
    })
}

async fn require_profile(state: &AppState, profile_id: &str) -> Result<RegistryProfile, AppError> {
    let id = Uuid::parse_str(profile_id)?;
    get_registry_profile(&state.pool, id).await?.ok_or_else(|| {
        AppError::new(
            "profile_not_found",
            "Selected registry profile was not found.",
        )
    })
}

async fn affected_tags(
    state: &AppState,
    profile: &RegistryProfile,
    client: &RegistryClient,
    repository: &str,
    digest: &str,
) -> Result<Vec<String>, AppError> {
    let mut tags = BTreeSet::new();
    for cache in list_manifest_cache_by_digest(&state.pool, profile.id, repository, digest).await? {
        tags.insert(cache.tag);
    }

    if let Ok(remote_tags) = client.list_tags(repository, Some(200), None).await {
        for tag in remote_tags.tags {
            if client
                .resolve_digest(repository, &tag)
                .await
                .map(|value| value == digest)
                .unwrap_or(false)
            {
                tags.insert(tag);
            }
        }
    }

    if tags.is_empty() {
        for cache in list_manifest_cache(&state.pool, profile.id, repository).await? {
            if cache.digest == digest {
                tags.insert(cache.tag);
            }
        }
    }

    Ok(tags.into_iter().collect())
}

fn digest_suffix(digest: &str) -> String {
    digest
        .chars()
        .rev()
        .take(12)
        .collect::<String>()
        .chars()
        .rev()
        .collect()
}

fn delete_error_code(error: &RegistryError) -> &'static str {
    match error {
        RegistryError::NotFound => "manifest_not_found",
        RegistryError::Unauthorized => "registry_unauthorized",
        RegistryError::Forbidden => "registry_forbidden",
        _ => "manifest_delete_failed",
    }
}

fn delete_error_message(error: &RegistryError) -> String {
    match error {
        RegistryError::NotFound => "Manifest digest was not found in the registry.".to_string(),
        RegistryError::Unauthorized => {
            "Registry returned 401; authenticate before deleting this digest.".to_string()
        }
        RegistryError::Forbidden => {
            "Registry returned 403; deletion is forbidden for this digest.".to_string()
        }
        _ => format!("Manifest delete failed: {error}"),
    }
}
