use std::collections::{BTreeMap, BTreeSet};
use std::time::{Duration, Instant};

use chrono::Utc;
use serde::Serialize;
use tauri::State;
use tokio::time::timeout;
use uuid::Uuid;

use crate::audit::{log_audit_event, AuditAction, AuditEvent};
use crate::registry::{RegistryClient, RegistryError, DOCKER_MANIFEST_LIST, OCI_IMAGE_INDEX};
use crate::store::{
    delete_repository_cache, get_registry_profile, list_manifest_cache,
    list_manifest_cache_by_digest, update_manifest_gc_status, upsert_repository_cache,
    RegistryProfile, RepositoryCache,
};

use super::{
    registry::ensure_local_registry_target, registry::registry_client_for_profile, AppError,
    AppState,
};

const REQUEST_TIMEOUT: Duration = if cfg!(test) {
    Duration::from_millis(200)
} else {
    Duration::from_secs(10)
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

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteRepositoryTagResult {
    pub tag: String,
    pub digest: Option<String>,
    pub status: String,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteRepositoryDigestResult {
    pub digest: String,
    pub tags: Vec<String>,
    pub status: String,
    pub pending_gc: bool,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteRepositoryResult {
    pub repository: String,
    pub status: String,
    pub total_tags: usize,
    pub total_digests: usize,
    pub deleted_digests: Vec<String>,
    pub failed_digests: Vec<DeleteRepositoryDigestResult>,
    pub tag_results: Vec<DeleteRepositoryTagResult>,
    pub digest_results: Vec<DeleteRepositoryDigestResult>,
    pub pending_gc: bool,
}

struct RepositoryDeleteAudit<'a> {
    profile: &'a RegistryProfile,
    repository: &'a str,
    digest: &'a str,
    tags: &'a [String],
    status: &'a str,
    error_message: Option<String>,
    duration_ms: i64,
}

#[tauri::command]
pub async fn get_delete_impact(
    profile_id: String,
    repository: String,
    reference: String,
    state: State<'_, AppState>,
) -> Result<DeleteImpact, AppError> {
    let profile = require_profile(&state, &profile_id).await?;
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
            container_id: profile.container_id.clone(),
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

#[tauri::command]
pub async fn delete_repository(
    profile_id: String,
    repository: String,
    state: State<'_, AppState>,
) -> Result<DeleteRepositoryResult, AppError> {
    delete_repository_for_state(&profile_id, repository, &state).await
}

async fn delete_repository_for_state(
    profile_id: &str,
    repository: String,
    state: &AppState,
) -> Result<DeleteRepositoryResult, AppError> {
    let profile = require_profile(state, profile_id).await?;
    ensure_local_registry_target(&profile.registry_url).await?;
    let client = registry_client_for_profile(&profile)?;
    let started = Instant::now();
    let tags = match timeout(REQUEST_TIMEOUT, client.list_tags(&repository, None, None)).await {
        Ok(Ok(tags)) => tags.tags,
        Ok(Err(error)) => return Err(AppError::from(error)),
        Err(_) => return Err(registry_timeout_error("list repository tags")),
    };
    let total_tags = tags.len();
    let mut tag_results = Vec::with_capacity(total_tags);
    let mut tags_by_digest = BTreeMap::<String, Vec<String>>::new();

    for tag in tags {
        match timeout(
            REQUEST_TIMEOUT,
            client.resolve_tag_digest(&repository, &tag),
        )
        .await
        {
            Ok(Ok(resolved)) => {
                tags_by_digest
                    .entry(resolved.digest.clone())
                    .or_default()
                    .push(resolved.tag.clone());
                tag_results.push(DeleteRepositoryTagResult {
                    tag: resolved.tag,
                    digest: Some(resolved.digest),
                    status: "resolved".to_string(),
                    error: None,
                });
            }
            Ok(Err(error)) => tag_results.push(DeleteRepositoryTagResult {
                tag,
                digest: None,
                status: "failure".to_string(),
                error: Some(delete_error_message(&error)),
            }),
            Err(_) => tag_results.push(DeleteRepositoryTagResult {
                tag,
                digest: None,
                status: "failure".to_string(),
                error: Some(registry_timeout_message("resolve tag digest")),
            }),
        }
    }

    let total_digests = tags_by_digest.len();
    let mut deleted_digests = Vec::new();
    let mut failed_digests = Vec::new();
    let mut digest_results = Vec::with_capacity(total_digests);

    for (digest, tags) in tags_by_digest {
        let result = match timeout(
            REQUEST_TIMEOUT,
            client.delete_manifest(&repository, &digest),
        )
        .await
        {
            Ok(Ok(())) => Ok(()),
            Ok(Err(error)) => Err(delete_error_message(&error)),
            Err(_) => Err(registry_timeout_message("delete manifest")),
        };
        let (status, error_message, pending_gc) = match &result {
            Ok(()) => ("pending_gc".to_string(), None, true),
            Err(error) => ("failure".to_string(), Some(error.clone()), false),
        };

        log_repository_delete_audit(
            state,
            RepositoryDeleteAudit {
                profile: &profile,
                repository: &repository,
                digest: &digest,
                tags: &tags,
                status: &status,
                error_message: error_message.clone(),
                duration_ms: started.elapsed().as_millis() as i64,
            },
        )
        .await?;

        let digest_result = DeleteRepositoryDigestResult {
            digest: digest.clone(),
            tags: tags.clone(),
            status,
            pending_gc,
            error: error_message,
        };

        if result.is_ok() {
            update_manifest_gc_status(&state.pool, profile.id, &repository, &digest, "pending_gc")
                .await?;
            deleted_digests.push(digest);
        } else {
            failed_digests.push(digest_result.clone());
        }
        digest_results.push(digest_result);
    }

    update_repository_delete_cache(state, &profile, &repository, &tag_results, &digest_results)
        .await?;

    Ok(DeleteRepositoryResult {
        repository,
        status: repository_delete_status(&tag_results, &digest_results).to_string(),
        total_tags,
        total_digests,
        pending_gc: !deleted_digests.is_empty(),
        deleted_digests,
        failed_digests,
        tag_results,
        digest_results,
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

async fn log_repository_delete_audit(
    state: &AppState,
    audit: RepositoryDeleteAudit<'_>,
) -> Result<(), AppError> {
    log_audit_event(
        &state.pool,
        &AuditEvent {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            action: AuditAction::DeleteManifest,
            registry_id: Some(audit.profile.id),
            container_id: audit.profile.container_id.clone(),
            repository_name: Some(audit.repository.to_string()),
            tag: Some(audit.tags.join(",")),
            digest: Some(audit.digest.to_string()),
            status: audit.status.to_string(),
            duration_ms: Some(audit.duration_ms),
            error_message: audit.error_message,
            log_excerpt: Some(
                serde_json::json!({
                    "registry": &audit.profile.registry_url,
                    "repository": audit.repository,
                    "tags": audit.tags,
                    "result": audit.status,
                })
                .to_string(),
            ),
        },
    )
    .await?;
    Ok(())
}

async fn update_repository_delete_cache(
    state: &AppState,
    profile: &RegistryProfile,
    repository: &str,
    tag_results: &[DeleteRepositoryTagResult],
    digest_results: &[DeleteRepositoryDigestResult],
) -> Result<(), AppError> {
    let failed_digests = digest_results
        .iter()
        .filter(|result| result.status == "failure")
        .map(|result| result.digest.as_str())
        .collect::<BTreeSet<_>>();
    let remaining_tags = tag_results
        .iter()
        .filter(|result| {
            result.status == "failure"
                || result
                    .digest
                    .as_deref()
                    .is_some_and(|digest| failed_digests.contains(digest))
        })
        .count() as i64;

    if remaining_tags == 0 {
        delete_repository_cache(&state.pool, profile.id, repository).await?;
        return Ok(());
    }

    upsert_repository_cache(
        &state.pool,
        &RepositoryCache {
            registry_id: profile.id,
            repository_name: repository.to_string(),
            tag_count: remaining_tags,
            last_synced_at: Some(Utc::now()),
            sync_status: "delete_partial".to_string(),
        },
    )
    .await?;
    Ok(())
}

fn repository_delete_status(
    tag_results: &[DeleteRepositoryTagResult],
    digest_results: &[DeleteRepositoryDigestResult],
) -> &'static str {
    let has_success = digest_results.iter().any(|result| result.pending_gc);
    let has_failure = tag_results.iter().any(|result| result.status == "failure")
        || digest_results
            .iter()
            .any(|result| result.status == "failure");

    match (has_success, has_failure) {
        (true, true) => "partial_failure",
        (true, false) => "pending_gc",
        (false, true) => "failure",
        (false, false) => "success",
    }
}

fn delete_error_code(error: &RegistryError) -> &'static str {
    match error {
        RegistryError::NotFound => "manifest_not_found",
        RegistryError::Unauthorized => "registry_unauthorized",
        RegistryError::Forbidden => "registry_forbidden",
        _ => "manifest_delete_failed",
    }
}

fn registry_timeout_error(operation: &str) -> AppError {
    AppError::new(
        "registry_request_timeout",
        registry_timeout_message(operation),
    )
}

fn registry_timeout_message(operation: &str) -> String {
    format!("Registry request timed out while trying to {operation}.")
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

#[cfg(test)]
mod tests {
    use std::io::{Read, Write};
    use std::net::{TcpListener, TcpStream};
    use std::{thread, time::Duration};

    use chrono::Utc;
    use uuid::Uuid;

    use super::delete_repository_for_state;
    use crate::audit::list_audit_events;
    use crate::commands::AppState;
    use crate::store::{
        list_manifest_cache_by_digest, list_repository_cache, save_registry_profile,
        upsert_manifest_cache, upsert_repository_cache, ManifestCache, RegistryProfile,
        RepositoryCache,
    };

    const DIGEST_A: &str =
        "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    const DIGEST_B: &str =
        "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

    #[tokio::test]
    async fn delete_repository_deduplicates_digest_deletes_and_prunes_cache() {
        let mock = spawn_delete_repository_mock(DeleteMockMode::AllSuccess);
        let state = AppState::in_memory()
            .await
            .expect("in-memory state should initialize");
        let profile = seed_profile(&state, &mock.base_url).await;
        seed_repository_cache(&state, &profile, "alpine", 3).await;
        seed_manifest_cache(&state, &profile, "alpine", "latest", DIGEST_A).await;
        seed_manifest_cache(&state, &profile, "alpine", "stable", DIGEST_A).await;

        let result =
            delete_repository_for_state(&profile.id.to_string(), "alpine".to_string(), &state)
                .await
                .expect("repository delete should succeed");
        let requests = mock.join();

        assert_eq!(result.status, "pending_gc");
        assert_eq!(result.total_tags, 3);
        assert_eq!(result.total_digests, 2);
        assert_eq!(result.deleted_digests, vec![DIGEST_A, DIGEST_B]);
        assert!(result.failed_digests.is_empty());
        assert_eq!(count_requests(&requests, "DELETE", DIGEST_A), 1);
        assert_eq!(count_requests(&requests, "DELETE", DIGEST_B), 1);

        let cached_repositories = list_repository_cache(&state.pool, profile.id)
            .await
            .expect("repository cache should load");
        assert!(cached_repositories.is_empty());

        let pending = list_manifest_cache_by_digest(&state.pool, profile.id, "alpine", DIGEST_A)
            .await
            .expect("manifest cache should load");
        assert!(pending
            .iter()
            .all(|manifest| manifest.gc_status.as_deref() == Some("pending_gc")));
    }

    #[tokio::test]
    async fn delete_repository_continues_after_digest_failure_and_reports_partial_result() {
        let mock = spawn_delete_repository_mock(DeleteMockMode::SecondDeleteFails);
        let state = AppState::in_memory()
            .await
            .expect("in-memory state should initialize");
        let profile = seed_profile(&state, &mock.base_url).await;
        seed_repository_cache(&state, &profile, "alpine", 2).await;

        let result =
            delete_repository_for_state(&profile.id.to_string(), "alpine".to_string(), &state)
                .await
                .expect("partial repository delete should return summary");
        let requests = mock.join();

        assert_eq!(result.status, "partial_failure");
        assert_eq!(result.deleted_digests, vec![DIGEST_A]);
        assert_eq!(result.failed_digests.len(), 1);
        assert_eq!(result.failed_digests[0].digest, DIGEST_B);
        assert_eq!(count_requests(&requests, "DELETE", DIGEST_A), 1);
        assert_eq!(count_requests(&requests, "DELETE", DIGEST_B), 1);

        let cached_repositories = list_repository_cache(&state.pool, profile.id)
            .await
            .expect("repository cache should load");
        assert_eq!(cached_repositories.len(), 1);
        assert_eq!(cached_repositories[0].tag_count, 1);
        assert_eq!(cached_repositories[0].sync_status, "delete_partial");

        let audit_events = list_audit_events(&state.pool, 10, 0)
            .await
            .expect("audit events should load");
        assert_eq!(audit_events.len(), 2);
        assert!(audit_events.iter().any(|event| {
            event.digest.as_deref() == Some(DIGEST_A) && event.status == "pending_gc"
        }));
        assert!(audit_events.iter().any(|event| {
            event.digest.as_deref() == Some(DIGEST_B) && event.status == "failure"
        }));
    }

    #[tokio::test]
    async fn delete_repository_rejects_remote_registry_before_delete() {
        let state = AppState::in_memory()
            .await
            .expect("in-memory state should initialize");
        let profile = seed_profile(&state, "https://registry.example.com").await;

        let error =
            delete_repository_for_state(&profile.id.to_string(), "alpine".to_string(), &state)
                .await
                .expect_err("remote repository delete should be rejected");

        assert_eq!(error.code, "REMOTE_REGISTRY_NOT_ALLOWED");
    }

    #[tokio::test]
    async fn delete_repository_times_out_when_tag_listing_hangs() {
        let mock = spawn_delete_repository_mock(DeleteMockMode::TagListHangs);
        let state = AppState::in_memory()
            .await
            .expect("in-memory state should initialize");
        let profile = seed_profile(&state, &mock.base_url).await;

        let error = tokio::time::timeout(
            Duration::from_secs(1),
            delete_repository_for_state(&profile.id.to_string(), "alpine".to_string(), &state),
        )
        .await
        .expect("repository delete should rely on its internal timeout")
        .expect_err("hanging tag list should fail");
        let requests = mock.join();

        assert_eq!(error.code, "registry_request_timeout");
        assert_eq!(count_requests(&requests, "GET", "/tags/list"), 1);
    }

    #[tokio::test]
    async fn delete_repository_treats_null_tag_list_as_empty_repository() {
        let mock = spawn_delete_repository_mock(DeleteMockMode::TagListNull);
        let state = AppState::in_memory()
            .await
            .expect("in-memory state should initialize");
        let profile = seed_profile(&state, &mock.base_url).await;
        seed_repository_cache(&state, &profile, "alpine", 1).await;

        let result =
            delete_repository_for_state(&profile.id.to_string(), "alpine".to_string(), &state)
                .await
                .expect("null tag list should return an empty repository summary");
        let requests = mock.join();

        assert_eq!(result.status, "success");
        assert_eq!(result.total_tags, 0);
        assert_eq!(result.total_digests, 0);
        assert!(result.deleted_digests.is_empty());
        assert!(result.failed_digests.is_empty());
        assert_eq!(count_requests(&requests, "GET", "/tags/list"), 1);

        let cached_repositories = list_repository_cache(&state.pool, profile.id)
            .await
            .expect("repository cache should load");
        assert!(cached_repositories.is_empty());
    }

    #[tokio::test]
    async fn delete_repository_records_tag_resolution_timeout() {
        let mock = spawn_delete_repository_mock(DeleteMockMode::ResolveHangs);
        let state = AppState::in_memory()
            .await
            .expect("in-memory state should initialize");
        let profile = seed_profile(&state, &mock.base_url).await;

        let result = tokio::time::timeout(
            Duration::from_secs(1),
            delete_repository_for_state(&profile.id.to_string(), "alpine".to_string(), &state),
        )
        .await
        .expect("repository delete should rely on its internal timeout")
        .expect("tag resolution timeout should return a failure summary");
        let requests = mock.join();

        assert_eq!(result.status, "failure");
        assert_eq!(result.total_tags, 1);
        assert_eq!(result.total_digests, 0);
        assert_eq!(result.tag_results[0].status, "failure");
        assert!(result.tag_results[0]
            .error
            .as_deref()
            .is_some_and(|message| message.contains("timed out")));
        assert_eq!(count_requests(&requests, "HEAD", "latest"), 1);
    }

    #[tokio::test]
    async fn delete_repository_records_manifest_delete_timeout() {
        let mock = spawn_delete_repository_mock(DeleteMockMode::DeleteHangs);
        let state = AppState::in_memory()
            .await
            .expect("in-memory state should initialize");
        let profile = seed_profile(&state, &mock.base_url).await;

        let result = tokio::time::timeout(
            Duration::from_secs(1),
            delete_repository_for_state(&profile.id.to_string(), "alpine".to_string(), &state),
        )
        .await
        .expect("repository delete should rely on its internal timeout")
        .expect("delete timeout should return a failure summary");
        let requests = mock.join();

        assert_eq!(result.status, "failure");
        assert_eq!(result.total_digests, 1);
        assert_eq!(result.failed_digests.len(), 1);
        assert!(result.failed_digests[0]
            .error
            .as_deref()
            .is_some_and(|message| message.contains("timed out")));
        assert_eq!(count_requests(&requests, "DELETE", DIGEST_A), 1);
    }

    struct DeleteMock {
        base_url: String,
        handle: thread::JoinHandle<Vec<String>>,
    }

    enum DeleteMockMode {
        AllSuccess,
        SecondDeleteFails,
        TagListHangs,
        TagListNull,
        ResolveHangs,
        DeleteHangs,
    }

    impl DeleteMock {
        fn join(self) -> Vec<String> {
            self.handle
                .join()
                .expect("mock registry thread should finish")
        }
    }

    fn spawn_delete_repository_mock(mode: DeleteMockMode) -> DeleteMock {
        let listener = TcpListener::bind("127.0.0.1:0").expect("mock registry should bind");
        let base_url = format!("http://{}", listener.local_addr().expect("mock address"));
        let handle = thread::spawn(move || {
            let mut requests = Vec::new();
            for _ in 0..request_count_for_mode(&mode) {
                let (mut stream, _) = listener.accept().expect("mock request should connect");
                let request = read_request(&mut stream);
                write_delete_response(&mut stream, &request, &mode);
                requests.push(request);
            }
            requests
        });

        DeleteMock { base_url, handle }
    }

    async fn seed_profile(state: &AppState, registry_url: &str) -> RegistryProfile {
        let now = Utc::now();
        let profile = RegistryProfile {
            id: Uuid::new_v4(),
            name: "Local".to_string(),
            registry_url: registry_url.to_string(),
            credential_ref: None,
            created_at: now,
            updated_at: now,
            container_id: None,
            container_name: None,
            config_path: None,
        };
        save_registry_profile(&state.pool, &profile)
            .await
            .expect("profile should save");
        profile
    }

    async fn seed_repository_cache(
        state: &AppState,
        profile: &RegistryProfile,
        repository: &str,
        tag_count: i64,
    ) {
        upsert_repository_cache(
            &state.pool,
            &RepositoryCache {
                registry_id: profile.id,
                repository_name: repository.to_string(),
                tag_count,
                last_synced_at: Some(Utc::now()),
                sync_status: "fresh".to_string(),
            },
        )
        .await
        .expect("repository cache should seed");
    }

    async fn seed_manifest_cache(
        state: &AppState,
        profile: &RegistryProfile,
        repository: &str,
        tag: &str,
        digest: &str,
    ) {
        upsert_manifest_cache(
            &state.pool,
            &ManifestCache {
                registry_id: profile.id,
                repository_name: repository.to_string(),
                tag: tag.to_string(),
                digest: digest.to_string(),
                media_type: "application/vnd.docker.distribution.manifest.v2+json".to_string(),
                platform_summary: None,
                raw_json: "{}".to_string(),
                last_synced_at: Utc::now(),
                gc_status: None,
            },
        )
        .await
        .expect("manifest cache should seed");
    }

    fn read_request(stream: &mut TcpStream) -> String {
        let mut request_buffer = [0_u8; 2048];
        let bytes_read = stream
            .read(&mut request_buffer)
            .expect("mock request should read");
        String::from_utf8_lossy(&request_buffer[..bytes_read]).to_string()
    }

    fn write_delete_response(stream: &mut TcpStream, request: &str, mode: &DeleteMockMode) {
        if should_hang_response(request, mode) {
            thread::sleep(Duration::from_millis(350));
            return;
        }

        if request.starts_with("GET /v2/alpine/tags/list") {
            if matches!(mode, DeleteMockMode::TagListNull) {
                write_json_response(stream, 200, r#"{"name":"alpine","tags":null}"#);
                return;
            }
            if matches!(
                mode,
                DeleteMockMode::ResolveHangs | DeleteMockMode::DeleteHangs
            ) {
                write_json_response(stream, 200, r#"{"name":"alpine","tags":["latest"]}"#);
                return;
            }
            write_json_response(
                stream,
                200,
                r#"{"name":"alpine","tags":["latest","stable","edge"]}"#,
            );
        } else if request.starts_with("HEAD /v2/alpine/manifests/latest")
            || request.starts_with("HEAD /v2/alpine/manifests/stable")
        {
            write_head_response(stream, 200, DIGEST_A);
        } else if request.starts_with("HEAD /v2/alpine/manifests/edge") {
            write_head_response(stream, 200, DIGEST_B);
        } else if request.starts_with(&format!("DELETE /v2/alpine/manifests/{DIGEST_A}")) {
            write_empty_response(stream, 202);
        } else if request.starts_with(&format!("DELETE /v2/alpine/manifests/{DIGEST_B}")) {
            match mode {
                DeleteMockMode::AllSuccess => write_empty_response(stream, 202),
                DeleteMockMode::SecondDeleteFails => write_empty_response(stream, 500),
                DeleteMockMode::TagListHangs
                | DeleteMockMode::TagListNull
                | DeleteMockMode::ResolveHangs
                | DeleteMockMode::DeleteHangs => write_empty_response(stream, 202),
            }
        } else {
            write_empty_response(stream, 404);
        }
    }

    fn request_count_for_mode(mode: &DeleteMockMode) -> usize {
        match mode {
            DeleteMockMode::AllSuccess | DeleteMockMode::SecondDeleteFails => 6,
            DeleteMockMode::TagListHangs | DeleteMockMode::TagListNull => 1,
            DeleteMockMode::ResolveHangs => 2,
            DeleteMockMode::DeleteHangs => 3,
        }
    }

    fn should_hang_response(request: &str, mode: &DeleteMockMode) -> bool {
        match mode {
            DeleteMockMode::TagListHangs => request.starts_with("GET /v2/alpine/tags/list"),
            DeleteMockMode::TagListNull => false,
            DeleteMockMode::ResolveHangs => request.starts_with("HEAD /v2/alpine/manifests/latest"),
            DeleteMockMode::DeleteHangs => {
                request.starts_with(&format!("DELETE /v2/alpine/manifests/{DIGEST_A}"))
            }
            DeleteMockMode::AllSuccess | DeleteMockMode::SecondDeleteFails => false,
        }
    }

    fn write_json_response(stream: &mut TcpStream, status: u16, body: &str) {
        let response = format!(
            "HTTP/1.1 {status} OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            body.len(),
            body
        );
        stream
            .write_all(response.as_bytes())
            .expect("mock response should write");
    }

    fn write_head_response(stream: &mut TcpStream, status: u16, digest: &str) {
        let response = format!(
            "HTTP/1.1 {status} OK\r\nDocker-Content-Digest: {digest}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
        );
        stream
            .write_all(response.as_bytes())
            .expect("mock response should write");
    }

    fn write_empty_response(stream: &mut TcpStream, status: u16) {
        let response =
            format!("HTTP/1.1 {status} OK\r\nContent-Length: 0\r\nConnection: close\r\n\r\n");
        stream
            .write_all(response.as_bytes())
            .expect("mock response should write");
    }

    fn count_requests(requests: &[String], method: &str, contains: &str) -> usize {
        requests
            .iter()
            .filter(|request| request.starts_with(method) && request.contains(contains))
            .count()
    }
}
