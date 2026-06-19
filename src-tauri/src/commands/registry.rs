use std::net::IpAddr;
use std::time::Duration;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tauri::State;
use tokio::time::timeout;
use url::Url;
use uuid::Uuid;

use crate::credentials::{CredentialStore, RegistryCredential, SystemKeyring};
use crate::registry::{LayerSummary, Manifest, PlatformSummary, RegistryClient, RegistryError};
use crate::store::{
    delete_registry_profile as remove_registry_profile, delete_repository_cache,
    get_registry_profile as load_registry_profile, get_registry_profile_by_url,
    get_selected_registry_profile as load_selected, list_manifest_cache,
    list_registry_profiles as load_registry_profiles, list_repository_cache, save_registry_profile,
    select_registry_profile as mark_registry_profile_selected, upsert_manifest_cache,
    upsert_repository_cache, ManifestCache, RegistryProfile, RepositoryCache,
};

use super::{AppError, AppState};

const REQUEST_TIMEOUT: Duration = if cfg!(test) {
    Duration::from_millis(200)
} else {
    Duration::from_secs(10)
};
const REFRESH_TIMEOUT: Duration = Duration::from_secs(30);
const PAGE_SIZE: u32 = 25;
const TAG_COUNT_SCAN_LIMIT: u32 = 100;
const SYNC_STATUS_FRESH: &str = "fresh";
const SYNC_STATUS_TAG_COUNT_STALE: &str = "tag_count_stale";

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RegistryProfileInput {
    pub name: Option<String>,
    pub registry_url: String,
    pub credential_ref: Option<String>,
    pub container_id: Option<String>,
    pub container_name: Option<String>,
    pub config_path: Option<String>,
}

struct NormalizedRegistryProfileInput {
    name: Option<String>,
    registry_url: String,
    credential_ref: Option<String>,
    container_name: Option<String>,
    config_path: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RegistryHealth {
    pub reachable: bool,
    pub status: String,
    pub message: String,
    pub checked_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CatalogPage {
    pub repositories: Vec<RepositoryCache>,
    pub next_last: Option<String>,
    pub stale: bool,
    pub last_synced_at: Option<DateTime<Utc>>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TagsPage {
    pub repository: String,
    pub tags: Vec<ManifestCache>,
    pub next_last: Option<String>,
    pub stale: bool,
    pub last_synced_at: Option<DateTime<Utc>>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ManifestSummary {
    pub repository: String,
    pub reference: String,
    pub digest: String,
    pub media_type: String,
    pub layers: Vec<LayerSummaryDto>,
    pub platforms: Vec<PlatformSummaryDto>,
    pub raw_json: String,
    pub size: usize,
    pub stale: bool,
    pub last_synced_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LayerSummaryDto {
    pub digest: String,
    pub size: i64,
    pub media_type: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlatformSummaryDto {
    pub os: Option<String>,
    pub architecture: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RefreshResult {
    pub profile_id: String,
    pub refreshed_repositories: usize,
    pub cancelled: bool,
    pub timed_out: bool,
}

#[tauri::command]
pub async fn list_registry_profiles(
    state: State<'_, AppState>,
) -> Result<Vec<RegistryProfile>, AppError> {
    list_registry_profiles_for_state(&state).await
}

#[tauri::command]
pub async fn create_registry_profile(
    profile: RegistryProfileInput,
    state: State<'_, AppState>,
) -> Result<RegistryProfile, AppError> {
    create_registry_profile_for_state(profile, &state).await
}

#[tauri::command]
pub async fn update_registry_profile(
    profile_id: String,
    profile: RegistryProfileInput,
    state: State<'_, AppState>,
) -> Result<RegistryProfile, AppError> {
    update_registry_profile_for_state(&profile_id, profile, &state).await
}

#[tauri::command]
pub async fn delete_registry_profile(
    profile_id: String,
    state: State<'_, AppState>,
) -> Result<bool, AppError> {
    delete_registry_profile_for_state(&profile_id, &state).await
}

#[tauri::command]
pub async fn select_registry_profile(
    profile: RegistryProfileInput,
    state: State<'_, AppState>,
) -> Result<RegistryProfile, AppError> {
    select_registry_profile_for_state(profile, &state).await
}

#[tauri::command]
pub async fn set_registry_credentials(
    profile_id: String,
    username: String,
    password: String,
    state: State<'_, AppState>,
) -> Result<(), AppError> {
    let profile = require_profile(&state, &profile_id).await?;
    let username = username.trim().to_string();
    if username.is_empty() {
        return Err(AppError::new(
            "invalid_registry_credentials",
            "Registry credential username is required.",
        ));
    }

    SystemKeyring.save(
        &profile.credential_lookup_key(),
        &RegistryCredential {
            username,
            secret: password,
        },
    )?;
    Ok(())
}

#[tauri::command]
pub async fn clear_registry_credentials(
    profile_id: String,
    state: State<'_, AppState>,
) -> Result<(), AppError> {
    let profile = require_profile(&state, &profile_id).await?;
    SystemKeyring.delete(&profile.credential_lookup_key())?;
    Ok(())
}

#[tauri::command]
pub async fn get_selected_registry_profile(
    state: State<'_, AppState>,
) -> Result<Option<RegistryProfile>, AppError> {
    Ok(load_selected(&state.pool).await?)
}

#[tauri::command]
pub async fn check_registry_health(
    profile_id: String,
    state: State<'_, AppState>,
) -> Result<RegistryHealth, AppError> {
    check_registry_health_for_state(&profile_id, &state).await
}

async fn check_registry_health_for_state(
    profile_id: &str,
    state: &AppState,
) -> Result<RegistryHealth, AppError> {
    let profile = require_profile(state, profile_id).await?;
    let checked_at = Utc::now();
    let client = registry_client_for_profile(&profile)?;
    match timeout(REQUEST_TIMEOUT, client.ping()).await {
        Ok(Ok(())) => Ok(RegistryHealth {
            reachable: true,
            status: "ok".to_string(),
            message: "/v2/ responded successfully.".to_string(),
            checked_at,
        }),
        Ok(Err(RegistryError::UnexpectedStatus(status))) => Ok(RegistryHealth {
            reachable: false,
            status: "registry_api_error".to_string(),
            message: format!("/v2/ returned HTTP {status}."),
            checked_at,
        }),
        Ok(Err(error)) => Ok(RegistryHealth {
            reachable: false,
            status: "v2_unavailable".to_string(),
            message: error.to_string(),
            checked_at,
        }),
        Err(_) => Ok(RegistryHealth {
            reachable: false,
            status: "timeout".to_string(),
            message: "/v2/ health check timed out.".to_string(),
            checked_at,
        }),
    }
}

#[tauri::command]
pub async fn list_catalog(
    profile_id: String,
    n: Option<u32>,
    last: Option<String>,
    state: State<'_, AppState>,
) -> Result<CatalogPage, AppError> {
    list_catalog_for_state(&profile_id, n, last, &state).await
}

async fn list_catalog_for_state(
    profile_id: &str,
    n: Option<u32>,
    last: Option<String>,
    state: &AppState,
) -> Result<CatalogPage, AppError> {
    let profile = require_profile(state, profile_id).await?;
    let client = registry_client_for_profile(&profile)?;
    let page_size = n.unwrap_or(PAGE_SIZE).min(PAGE_SIZE);

    match timeout(REQUEST_TIMEOUT, client.list_catalog(Some(page_size), last)).await {
        Ok(Ok(catalog)) => {
            let synced_at = Utc::now();
            let catalog_repositories = catalog.repositories;
            let next_last = if catalog_repositories.len() == page_size as usize {
                catalog_repositories.last().cloned()
            } else {
                None
            };
            let mut repositories = Vec::with_capacity(catalog_repositories.len());
            for repository_name in catalog_repositories {
                let (tag_count, sync_status) =
                    repository_tag_count(&state.pool, profile.id, &client, &repository_name)
                        .await?;
                let cache = RepositoryCache {
                    registry_id: profile.id,
                    repository_name,
                    tag_count,
                    last_synced_at: Some(synced_at),
                    sync_status: sync_status.to_string(),
                };

                if !cache.has_tags() {
                    prune_zero_tag_repository_cache(
                        &state.pool,
                        profile.id,
                        &cache.repository_name,
                        sync_status,
                    )
                    .await?;
                    continue;
                }

                upsert_repository_cache(&state.pool, &cache).await?;
                repositories.push(cache);
            }

            Ok(CatalogPage {
                repositories,
                next_last,
                stale: false,
                last_synced_at: Some(synced_at),
                error: None,
            })
        }
        Ok(Err(error)) => cached_catalog_page(state, profile.id, error.to_string()).await,
        Err(_) => {
            cached_catalog_page(state, profile.id, "catalog request timed out".to_string()).await
        }
    }
}

#[tauri::command]
pub async fn list_tags(
    profile_id: String,
    repository: String,
    n: Option<u32>,
    last: Option<String>,
    state: State<'_, AppState>,
) -> Result<TagsPage, AppError> {
    let profile = require_profile(&state, &profile_id).await?;
    let client = registry_client_for_profile(&profile)?;
    let page_size = n.unwrap_or(PAGE_SIZE);

    match timeout(
        REQUEST_TIMEOUT,
        client.list_tags(&repository, Some(page_size), last),
    )
    .await
    {
        Ok(Ok(tags)) => {
            let mut cached_tags = Vec::with_capacity(tags.tags.len());
            let synced_at = Utc::now();
            for tag in tags.tags {
                let cache =
                    manifest_cache_from_reference(&profile, &repository, &tag, synced_at, &client)
                        .await?;
                upsert_manifest_cache(&state.pool, &cache).await?;
                cached_tags.push(cache);
            }
            let existing = list_manifest_cache(&state.pool, profile.id, &repository).await?;
            let repo_cache = RepositoryCache {
                registry_id: profile.id,
                repository_name: repository.clone(),
                tag_count: existing.len() as i64,
                last_synced_at: Some(synced_at),
                sync_status: SYNC_STATUS_FRESH.to_string(),
            };
            upsert_repository_cache(&state.pool, &repo_cache).await?;

            Ok(TagsPage {
                repository,
                next_last: next_cursor(&cached_tags, page_size, |tag| &tag.tag),
                tags: cached_tags,
                stale: false,
                last_synced_at: Some(synced_at),
                error: None,
            })
        }
        Ok(Err(error)) => cached_tags_page(&state, profile.id, repository, error.to_string()).await,
        Err(_) => {
            cached_tags_page(
                &state,
                profile.id,
                repository,
                "tags request timed out".to_string(),
            )
            .await
        }
    }
}

#[tauri::command]
pub async fn get_manifest(
    profile_id: String,
    repository: String,
    reference: String,
    state: State<'_, AppState>,
) -> Result<ManifestSummary, AppError> {
    let profile = require_profile(&state, &profile_id).await?;
    let client = registry_client_for_profile(&profile)?;

    match timeout(
        REQUEST_TIMEOUT,
        fetch_manifest_summary(&client, &repository, &reference),
    )
    .await
    {
        Ok(Ok(summary)) => {
            let cache = ManifestCache {
                registry_id: profile.id,
                repository_name: repository,
                tag: reference,
                digest: summary.digest.clone(),
                media_type: summary.media_type.clone(),
                platform_summary: platform_label(&summary.platforms),
                raw_json: summary.raw_json.clone(),
                last_synced_at: summary.last_synced_at.unwrap_or_else(Utc::now),
                gc_status: None,
            };
            upsert_manifest_cache(&state.pool, &cache).await?;
            Ok(summary)
        }
        Ok(Err(error)) => {
            cached_manifest_summary(&state, profile.id, repository, reference, error).await
        }
        Err(_) => {
            cached_manifest_summary(
                &state,
                profile.id,
                repository,
                reference,
                RegistryError::UnexpectedStatus(408),
            )
            .await
        }
    }
}

#[tauri::command]
pub async fn refresh_registry(
    profile_id: String,
    state: State<'_, AppState>,
) -> Result<RefreshResult, AppError> {
    let profile = require_profile(&state, &profile_id).await?;
    let pool = state.pool.clone();
    let task_profile_id = profile_id.clone();
    let task = tokio::spawn(async move { refresh_profile_catalog(pool, profile).await });
    let abort_handle = task.abort_handle();
    state
        .refresh_tasks
        .lock()
        .expect("refresh task mutex should not be poisoned")
        .insert(profile_id.clone(), abort_handle);

    let result = match timeout(REFRESH_TIMEOUT, task).await {
        Ok(Ok(Ok(count))) => Ok(RefreshResult {
            profile_id: task_profile_id,
            refreshed_repositories: count,
            cancelled: false,
            timed_out: false,
        }),
        Ok(Ok(Err(error))) => Err(error),
        Ok(Err(join_error)) if join_error.is_cancelled() => Ok(RefreshResult {
            profile_id: task_profile_id,
            refreshed_repositories: 0,
            cancelled: true,
            timed_out: false,
        }),
        Ok(Err(join_error)) => Err(AppError::with_details(
            "refresh_failed",
            "Registry refresh task failed.",
            join_error.to_string(),
        )),
        Err(_) => {
            if let Some(handle) = state
                .refresh_tasks
                .lock()
                .expect("refresh task mutex should not be poisoned")
                .get(&profile_id)
            {
                handle.abort();
            }
            Ok(RefreshResult {
                profile_id: task_profile_id,
                refreshed_repositories: 0,
                cancelled: false,
                timed_out: true,
            })
        }
    };

    state
        .refresh_tasks
        .lock()
        .expect("refresh task mutex should not be poisoned")
        .remove(&profile_id);

    result
}

#[tauri::command]
pub async fn cancel_refresh(
    profile_id: String,
    state: State<'_, AppState>,
) -> Result<bool, AppError> {
    let handle = state
        .refresh_tasks
        .lock()
        .expect("refresh task mutex should not be poisoned")
        .remove(&profile_id);

    if let Some(handle) = handle {
        handle.abort();
        return Ok(true);
    }

    Ok(false)
}

async fn list_registry_profiles_for_state(
    state: &AppState,
) -> Result<Vec<RegistryProfile>, AppError> {
    Ok(load_registry_profiles(&state.pool).await?)
}

async fn create_registry_profile_for_state(
    profile: RegistryProfileInput,
    state: &AppState,
) -> Result<RegistryProfile, AppError> {
    let input = normalize_profile_input(profile)?;
    if let Some(existing) = get_registry_profile_by_url(&state.pool, &input.registry_url).await? {
        return Ok(existing);
    }

    let now = Utc::now();
    let profile = RegistryProfile {
        id: Uuid::new_v4(),
        name: input.name.unwrap_or_else(|| input.registry_url.clone()),
        registry_url: input.registry_url,
        credential_ref: input.credential_ref,
        created_at: now,
        updated_at: now,
        container_id: None,
        container_name: input.container_name,
        config_path: input.config_path,
    };

    save_registry_profile(&state.pool, &profile).await?;
    Ok(profile)
}

async fn update_registry_profile_for_state(
    profile_id: &str,
    profile: RegistryProfileInput,
    state: &AppState,
) -> Result<RegistryProfile, AppError> {
    let input = normalize_profile_input(profile)?;
    let mut existing = require_profile(state, profile_id).await?;
    if input.registry_url != existing.registry_url {
        if let Some(other) = get_registry_profile_by_url(&state.pool, &input.registry_url).await? {
            if other.id != existing.id {
                return Err(duplicate_registry_url_error());
            }
        }
    }
    existing.name = input.name.unwrap_or_else(|| input.registry_url.clone());
    existing.registry_url = input.registry_url;
    existing.credential_ref = input.credential_ref;
    existing.container_id = None;
    existing.container_name = input.container_name;
    if let Some(config_path) = input.config_path {
        existing.config_path = Some(config_path);
    }
    existing.updated_at = Utc::now();

    save_registry_profile(&state.pool, &existing).await?;
    Ok(existing)
}

async fn delete_registry_profile_for_state(
    profile_id: &str,
    state: &AppState,
) -> Result<bool, AppError> {
    let id = Uuid::parse_str(profile_id)?;
    Ok(remove_registry_profile(&state.pool, id).await?)
}

async fn select_registry_profile_for_state(
    profile: RegistryProfileInput,
    state: &AppState,
) -> Result<RegistryProfile, AppError> {
    let input = normalize_profile_input(profile)?;
    let selected_at = Utc::now();
    let profile = match get_registry_profile_by_url(&state.pool, &input.registry_url).await? {
        Some(mut existing) => {
            if let Some(name) = input.name {
                existing.name = name;
            }
            if let Some(credential_ref) = input.credential_ref {
                existing.credential_ref = Some(credential_ref);
            }
            existing.container_id = None;
            existing.container_name = input.container_name;
            if let Some(config_path) = input.config_path {
                existing.config_path = Some(config_path);
            }
            existing.updated_at = selected_at;
            save_registry_profile(&state.pool, &existing).await?;
            existing
        }
        None => {
            let new_profile = RegistryProfile {
                id: Uuid::new_v4(),
                name: input.name.unwrap_or_else(|| input.registry_url.clone()),
                registry_url: input.registry_url,
                credential_ref: input.credential_ref,
                created_at: selected_at,
                updated_at: selected_at,
                container_id: None,
                container_name: input.container_name,
                config_path: input.config_path,
            };
            save_registry_profile(&state.pool, &new_profile).await?;
            new_profile
        }
    };

    mark_registry_profile_selected(&state.pool, profile.id, selected_at).await?;
    load_registry_profile(&state.pool, profile.id)
        .await?
        .ok_or_else(profile_not_found_error)
}

async fn require_profile(state: &AppState, profile_id: &str) -> Result<RegistryProfile, AppError> {
    let id = Uuid::parse_str(profile_id)?;
    load_registry_profile(&state.pool, id)
        .await?
        .ok_or_else(profile_not_found_error)
}

fn normalize_profile_input(
    input: RegistryProfileInput,
) -> Result<NormalizedRegistryProfileInput, AppError> {
    Ok(NormalizedRegistryProfileInput {
        name: optional_trimmed(input.name),
        registry_url: normalize_registry_url(&input.registry_url)?,
        credential_ref: optional_trimmed(input.credential_ref),
        container_name: optional_trimmed(input.container_name),
        config_path: optional_trimmed(input.config_path),
    })
}

fn normalize_registry_url(value: &str) -> Result<String, AppError> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(AppError::new(
            "invalid_registry_url",
            "Registry URL is required.",
        ));
    }

    let url = Url::parse(trimmed)
        .map_err(|_| AppError::new("invalid_registry_url", "Invalid registry URL."))?;
    if !matches!(url.scheme(), "http" | "https") || url.host_str().is_none() {
        return Err(AppError::new(
            "invalid_registry_url",
            "Registry URL must be an HTTP(S) URL with a host.",
        ));
    }

    Ok(trimmed.trim_end_matches('/').to_string())
}

fn optional_trimmed(value: Option<String>) -> Option<String> {
    value.and_then(|value| {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    })
}

fn profile_not_found_error() -> AppError {
    AppError::new(
        "profile_not_found",
        "Selected registry profile was not found.",
    )
}

fn duplicate_registry_url_error() -> AppError {
    AppError::new(
        "duplicate_registry_url",
        "A registry profile with this URL already exists.",
    )
}

pub(crate) async fn ensure_local_registry_target(registry_url: &str) -> Result<(), AppError> {
    let url = Url::parse(registry_url)
        .map_err(|_| AppError::new("invalid_registry_url", "Invalid registry URL."))?;
    let host = url
        .host_str()
        .ok_or_else(|| AppError::new("invalid_registry_url", "Invalid registry URL."))?;

    if is_loopback_host(host) {
        return Ok(());
    }

    Err(remote_registry_not_allowed(registry_url))
}

pub(crate) fn registry_client_for_profile(
    profile: &RegistryProfile,
) -> Result<RegistryClient, AppError> {
    let client = RegistryClient::new(profile.registry_url.clone());
    let Some(credential) = SystemKeyring.load(&profile.credential_lookup_key())? else {
        return Ok(client);
    };

    Ok(client.with_basic_auth(credential.username, credential.secret))
}

fn is_loopback_host(host: &str) -> bool {
    let normalized = host
        .strip_prefix('[')
        .and_then(|value| value.strip_suffix(']'))
        .unwrap_or(host);

    normalized.eq_ignore_ascii_case("localhost")
        || normalized
            .parse::<IpAddr>()
            .map(|ip| ip.is_loopback())
            .unwrap_or(false)
}

fn remote_registry_not_allowed(registry_url: &str) -> AppError {
    AppError::with_details(
        "REMOTE_REGISTRY_NOT_ALLOWED",
        "Only local Docker registry targets are allowed.",
        format!("Registry URL is not loopback-local: {registry_url}"),
    )
}

async fn cached_catalog_page(
    state: &AppState,
    profile_id: Uuid,
    error: String,
) -> Result<CatalogPage, AppError> {
    let repositories = list_repository_cache(&state.pool, profile_id).await?;
    if repositories.is_empty() {
        return Err(AppError::with_details(
            "registry_unreachable",
            "Registry is unreachable and no cached repositories are available.",
            error,
        ));
    }
    Ok(CatalogPage {
        next_last: None,
        last_synced_at: repositories
            .iter()
            .filter_map(|repo| repo.last_synced_at)
            .max(),
        repositories,
        stale: true,
        error: Some(error),
    })
}

async fn cached_tags_page(
    state: &AppState,
    profile_id: Uuid,
    repository: String,
    error: String,
) -> Result<TagsPage, AppError> {
    let tags = list_manifest_cache(&state.pool, profile_id, &repository).await?;
    if tags.is_empty() {
        return Err(AppError::with_details(
            "registry_unreachable",
            "Registry is unreachable and no cached tags are available.",
            error,
        ));
    }
    Ok(TagsPage {
        repository,
        next_last: None,
        last_synced_at: tags.iter().map(|tag| tag.last_synced_at).max(),
        tags,
        stale: true,
        error: Some(error),
    })
}

async fn cached_manifest_summary(
    state: &AppState,
    profile_id: Uuid,
    repository: String,
    reference: String,
    error: RegistryError,
) -> Result<ManifestSummary, AppError> {
    let cached = list_manifest_cache(&state.pool, profile_id, &repository)
        .await?
        .into_iter()
        .find(|manifest| manifest.tag == reference || manifest.digest == reference);

    cached
        .map(|cache| ManifestSummary {
            repository,
            reference,
            digest: cache.digest,
            media_type: cache.media_type,
            layers: Vec::new(),
            platforms: platform_summary_from_cache(cache.platform_summary),
            size: cache.raw_json.len(),
            raw_json: cache.raw_json,
            stale: true,
            last_synced_at: Some(cache.last_synced_at),
        })
        .ok_or_else(|| AppError::from(error))
}

async fn manifest_cache_from_reference(
    profile: &RegistryProfile,
    repository: &str,
    tag: &str,
    synced_at: DateTime<Utc>,
    client: &RegistryClient,
) -> Result<ManifestCache, AppError> {
    let summary = fetch_manifest_summary(client, repository, tag).await?;
    Ok(ManifestCache {
        registry_id: profile.id,
        repository_name: repository.to_string(),
        tag: tag.to_string(),
        digest: summary.digest,
        media_type: summary.media_type,
        platform_summary: platform_label(&summary.platforms),
        raw_json: summary.raw_json,
        last_synced_at: synced_at,
        gc_status: None,
    })
}

async fn fetch_manifest_summary(
    client: &RegistryClient,
    repository: &str,
    reference: &str,
) -> Result<ManifestSummary, RegistryError> {
    let digest = client.resolve_digest(repository, reference).await?;
    let (media_type, bytes) = client.fetch_manifest_raw(repository, reference).await?;
    let raw_json = String::from_utf8_lossy(&bytes).to_string();
    let manifest = client.fetch_manifest(repository, reference).await?;
    let (layers, platforms) = summarize_manifest(manifest);

    Ok(ManifestSummary {
        repository: repository.to_string(),
        reference: reference.to_string(),
        digest,
        media_type,
        layers,
        platforms,
        size: bytes.len(),
        raw_json,
        stale: false,
        last_synced_at: Some(Utc::now()),
    })
}

async fn refresh_profile_catalog(
    pool: sqlx::SqlitePool,
    profile: RegistryProfile,
) -> Result<usize, AppError> {
    let client = registry_client_for_profile(&profile)?;
    let mut last = None;
    let mut refreshed = 0;

    loop {
        let catalog = client.list_catalog(Some(PAGE_SIZE), last.clone()).await?;
        if catalog.repositories.is_empty() {
            break;
        }

        let synced_at = Utc::now();
        for repository_name in &catalog.repositories {
            let (tag_count, sync_status) =
                repository_tag_count(&pool, profile.id, &client, repository_name).await?;
            if tag_count == 0 {
                prune_zero_tag_repository_cache(&pool, profile.id, repository_name, sync_status)
                    .await?;
                continue;
            }

            upsert_repository_cache(
                &pool,
                &RepositoryCache {
                    registry_id: profile.id,
                    repository_name: repository_name.clone(),
                    tag_count,
                    last_synced_at: Some(synced_at),
                    sync_status: sync_status.to_string(),
                },
            )
            .await?;
            refreshed += 1;
        }

        if catalog.repositories.len() < PAGE_SIZE as usize {
            break;
        }
        last = catalog.repositories.last().cloned();
    }

    Ok(refreshed)
}

async fn repository_tag_count(
    pool: &sqlx::SqlitePool,
    profile_id: Uuid,
    client: &RegistryClient,
    repository: &str,
) -> Result<(i64, &'static str), AppError> {
    match timeout(
        REQUEST_TIMEOUT,
        client.list_tags(repository, Some(TAG_COUNT_SCAN_LIMIT), None),
    )
    .await
    {
        Ok(Ok(tags)) => Ok((tags.tags.len() as i64, SYNC_STATUS_FRESH)),
        Ok(Err(_)) | Err(_) => Ok((
            list_manifest_cache(pool, profile_id, repository)
                .await?
                .len() as i64,
            SYNC_STATUS_TAG_COUNT_STALE,
        )),
    }
}

async fn prune_zero_tag_repository_cache(
    pool: &sqlx::SqlitePool,
    profile_id: Uuid,
    repository_name: &str,
    sync_status: &str,
) -> Result<(), AppError> {
    if sync_status == SYNC_STATUS_FRESH {
        delete_repository_cache(pool, profile_id, repository_name).await?;
    }

    Ok(())
}

fn summarize_manifest(manifest: Manifest) -> (Vec<LayerSummaryDto>, Vec<PlatformSummaryDto>) {
    match manifest {
        Manifest::DockerSchema2V1 { layers, .. }
        | Manifest::DockerSchema2V2 { layers, .. }
        | Manifest::OciImageManifest { layers, .. } => (
            layers.into_iter().map(LayerSummaryDto::from).collect(),
            Vec::new(),
        ),
        Manifest::DockerManifestList { platforms, .. }
        | Manifest::OciImageIndex { platforms, .. } => (
            Vec::new(),
            platforms
                .into_iter()
                .map(PlatformSummaryDto::from)
                .collect(),
        ),
        Manifest::Raw { .. } => (Vec::new(), Vec::new()),
    }
}

fn next_cursor<T>(items: &[T], page_size: u32, get_value: impl Fn(&T) -> &str) -> Option<String> {
    if items.len() == page_size as usize {
        items.last().map(|item| get_value(item).to_string())
    } else {
        None
    }
}

fn platform_label(platforms: &[PlatformSummaryDto]) -> Option<String> {
    if platforms.is_empty() {
        return None;
    }
    Some(
        platforms
            .iter()
            .map(|platform| {
                format!(
                    "{}/{}",
                    platform.os.as_deref().unwrap_or("unknown"),
                    platform.architecture.as_deref().unwrap_or("unknown")
                )
            })
            .collect::<Vec<_>>()
            .join(", "),
    )
}

fn platform_summary_from_cache(value: Option<String>) -> Vec<PlatformSummaryDto> {
    value
        .into_iter()
        .flat_map(|labels| {
            labels
                .split(',')
                .map(str::trim)
                .map(str::to_string)
                .collect::<Vec<_>>()
        })
        .filter(|label| !label.is_empty())
        .map(|label| {
            let mut parts = label.splitn(2, '/');
            PlatformSummaryDto {
                os: parts.next().map(str::to_string),
                architecture: parts.next().map(str::to_string),
            }
        })
        .collect()
}

impl From<LayerSummary> for LayerSummaryDto {
    fn from(value: LayerSummary) -> Self {
        Self {
            digest: value.digest,
            size: value.size,
            media_type: value.media_type,
        }
    }
}

impl From<PlatformSummary> for PlatformSummaryDto {
    fn from(value: PlatformSummary) -> Self {
        Self {
            os: value.os,
            architecture: value.architecture,
        }
    }
}

#[cfg(test)]
mod tests {
    use chrono::Utc;
    use std::io::{Read, Write};
    use std::net::{TcpListener, TcpStream};
    use std::thread;

    use super::{
        cached_tags_page, check_registry_health_for_state, create_registry_profile_for_state,
        delete_registry_profile_for_state, ensure_local_registry_target, list_catalog_for_state,
        list_registry_profiles_for_state, select_registry_profile_for_state,
        update_registry_profile_for_state, RegistryProfileInput,
    };
    use crate::commands::AppState;
    use crate::store::{
        get_registry_profile, get_selected_registry_profile, list_repository_cache,
        upsert_manifest_cache, upsert_repository_cache, ManifestCache, RepositoryCache,
    };

    const REMOTE_REGISTRY_URL: &str = "http://203.0.113.1:9";

    #[tokio::test]
    async fn local_registry_target_accepts_loopback_hosts() {
        for url in [
            "http://localhost:5000",
            "http://127.0.0.1:5000",
            "http://[::1]:5000",
        ] {
            ensure_local_registry_target(url)
                .await
                .expect("loopback registry URL should be accepted");
        }
    }

    #[tokio::test]
    async fn local_registry_target_rejects_remote_hosts() {
        for url in ["http://192.168.1.100:5000", "https://registry.example.com"] {
            let error = ensure_local_registry_target(url)
                .await
                .expect_err("remote registry URL should be rejected");

            assert_eq!(error.code, "REMOTE_REGISTRY_NOT_ALLOWED");
        }
    }

    #[tokio::test]
    async fn registry_profile_crud_list_and_select_round_trip() {
        let state = AppState::in_memory()
            .await
            .expect("in-memory state should initialize");

        let created = create_registry_profile_for_state(
            profile_input(
                "  Remote Registry  ",
                "https://registry.example.com/",
                Some(" cred-a "),
            ),
            &state,
        )
        .await
        .expect("profile should be created");
        assert_eq!(created.name, "Remote Registry");
        assert_eq!(created.registry_url, "https://registry.example.com");
        assert_eq!(created.credential_ref.as_deref(), Some("cred-a"));

        let updated = update_registry_profile_for_state(
            &created.id.to_string(),
            profile_input("Renamed Registry", "https://registry.example.org", None),
            &state,
        )
        .await
        .expect("profile should update");
        assert_eq!(updated.id, created.id);
        assert_eq!(updated.name, "Renamed Registry");
        assert_eq!(updated.registry_url, "https://registry.example.org");
        assert!(updated.credential_ref.is_none());

        let selected = select_registry_profile_for_state(
            profile_input("Selected Registry", "https://registry.example.org", None),
            &state,
        )
        .await
        .expect("selecting an existing URL should reuse the profile");
        assert_eq!(selected.id, created.id);
        assert_eq!(selected.name, "Selected Registry");

        let selected_from_store = get_selected_registry_profile(&state.pool)
            .await
            .expect("selected profile should load")
            .expect("selected profile should exist");
        assert_eq!(selected_from_store.id, created.id);

        let profiles = list_registry_profiles_for_state(&state)
            .await
            .expect("profiles should list");
        assert_eq!(profiles.len(), 1);

        assert!(
            delete_registry_profile_for_state(&created.id.to_string(), &state)
                .await
                .expect("profile should delete")
        );
        assert!(list_registry_profiles_for_state(&state)
            .await
            .expect("profiles should list after delete")
            .is_empty());
    }

    #[tokio::test]
    async fn create_registry_profile_reuses_existing_url() {
        let state = AppState::in_memory()
            .await
            .expect("in-memory state should initialize");

        let first = create_registry_profile_for_state(
            profile_input("First", "https://registry.example.com", None),
            &state,
        )
        .await
        .expect("first profile should be created");

        let existing = create_registry_profile_for_state(
            profile_input("Second", "https://registry.example.com/", None),
            &state,
        )
        .await
        .expect("duplicate URL should return the existing profile");

        assert_eq!(existing.id, first.id);
        assert_eq!(existing.name, "First");

        let profiles = list_registry_profiles_for_state(&state)
            .await
            .expect("profiles should list");
        assert_eq!(profiles.len(), 1);
    }

    #[tokio::test]
    async fn update_registry_profile_rejects_url_collision_with_other_profile() {
        let state = AppState::in_memory()
            .await
            .expect("in-memory state should initialize");

        let _first = create_registry_profile_for_state(
            profile_input("First", "https://registry.example.com", None),
            &state,
        )
        .await
        .expect("first profile should be created");

        let second = create_registry_profile_for_state(
            profile_input("Second", "https://registry.example.org", None),
            &state,
        )
        .await
        .expect("second profile should be created");

        let error = update_registry_profile_for_state(
            &second.id.to_string(),
            profile_input("Second Renamed", "https://registry.example.com", None),
            &state,
        )
        .await
        .expect_err("URL collision with another profile should be rejected");

        assert_eq!(error.code, "duplicate_registry_url");

        let unchanged = get_registry_profile(&state.pool, second.id)
            .await
            .expect("second profile should still load")
            .expect("second profile should still exist");
        assert_eq!(unchanged.registry_url, "https://registry.example.org");
    }

    #[tokio::test]
    async fn selecting_profile_does_not_persist_fixed_container_id() {
        let state = AppState::in_memory()
            .await
            .expect("in-memory state should initialize");

        let selected = select_registry_profile_for_state(
            RegistryProfileInput {
                name: Some("Manual Local".to_string()),
                registry_url: "http://localhost:5000".to_string(),
                credential_ref: None,
                container_id: Some("container-123".to_string()),
                container_name: Some("registry".to_string()),
                config_path: Some("/etc/docker/registry/config.yml".to_string()),
            },
            &state,
        )
        .await
        .expect("container-linked profile should select");

        assert!(selected.container_id.is_none());
        assert_eq!(selected.container_name.as_deref(), Some("registry"));

        let stored = get_selected_registry_profile(&state.pool)
            .await
            .expect("selected profile should load")
            .expect("selected profile should exist");
        assert!(stored.container_id.is_none());
        assert_eq!(stored.container_name.as_deref(), Some("registry"));
    }

    #[tokio::test]
    async fn check_registry_health_allows_remote_urls_without_persisting_status() {
        let state = AppState::in_memory()
            .await
            .expect("in-memory state should initialize");
        let profile = select_registry_profile_for_state(
            profile_input("Remote", REMOTE_REGISTRY_URL, None),
            &state,
        )
        .await
        .expect("remote profile should save");
        let before = get_registry_profile(&state.pool, profile.id)
            .await
            .expect("profile should load before health check")
            .expect("profile should exist before health check");

        let health = check_registry_health_for_state(&profile.id.to_string(), &state)
            .await
            .expect("remote health checks should return live status, not local-only errors");
        assert!(!health.reachable);

        let after = get_registry_profile(&state.pool, profile.id)
            .await
            .expect("profile should load after health check")
            .expect("profile should exist after health check");
        assert_eq!(after, before);
    }

    #[tokio::test]
    async fn remote_catalog_reads_can_fall_back_to_cache() {
        let state = AppState::in_memory()
            .await
            .expect("in-memory state should initialize");
        let profile = select_registry_profile_for_state(
            profile_input("Remote", REMOTE_REGISTRY_URL, None),
            &state,
        )
        .await
        .expect("remote profile should save");
        let synced_at = Utc::now();
        upsert_repository_cache(
            &state.pool,
            &RepositoryCache {
                registry_id: profile.id,
                repository_name: "alpine".to_string(),
                tag_count: 1,
                last_synced_at: Some(synced_at),
                sync_status: "cached".to_string(),
            },
        )
        .await
        .expect("repository cache should seed");

        let page = list_catalog_for_state(&profile.id.to_string(), Some(25), None, &state)
            .await
            .expect("remote read should not be rejected before cache fallback");

        assert!(page.stale);
        assert_eq!(page.last_synced_at, Some(synced_at));
        assert!(page.error.is_some());
        assert_eq!(page.repositories[0].repository_name, "alpine");
    }

    #[tokio::test]
    async fn cached_tags_page_marks_cache_stale_with_last_sync() {
        let state = AppState::in_memory()
            .await
            .expect("in-memory state should initialize");
        let profile = select_registry_profile_for_state(
            profile_input("Remote", REMOTE_REGISTRY_URL, None),
            &state,
        )
        .await
        .expect("remote profile should save");
        let synced_at = Utc::now();
        upsert_manifest_cache(
            &state.pool,
            &ManifestCache {
                registry_id: profile.id,
                repository_name: "alpine".to_string(),
                tag: "latest".to_string(),
                digest: "sha256:test".to_string(),
                media_type: "application/vnd.docker.distribution.manifest.v2+json".to_string(),
                platform_summary: None,
                raw_json: "{}".to_string(),
                last_synced_at: synced_at,
                gc_status: None,
            },
        )
        .await
        .expect("tag cache should seed");

        let page = cached_tags_page(
            &state,
            profile.id,
            "alpine".to_string(),
            "tags request timed out".to_string(),
        )
        .await
        .expect("cached tags should load");

        assert!(page.stale);
        assert_eq!(page.last_synced_at, Some(synced_at));
        assert_eq!(page.error.as_deref(), Some("tags request timed out"));
        assert_eq!(page.tags[0].tag, "latest");
    }

    #[tokio::test]
    async fn list_catalog_excludes_zero_tag_repositories_from_live_result_and_cache() {
        let (registry_url, handle) = spawn_catalog_with_tag_counts_mock();
        let state = AppState::in_memory()
            .await
            .expect("in-memory state should initialize");
        let profile =
            select_registry_profile_for_state(profile_input("Local", &registry_url, None), &state)
                .await
                .expect("profile should save");
        upsert_repository_cache(
            &state.pool,
            &RepositoryCache {
                registry_id: profile.id,
                repository_name: "empty".to_string(),
                tag_count: 1,
                last_synced_at: Some(Utc::now()),
                sync_status: "fresh".to_string(),
            },
        )
        .await
        .expect("stale repository cache should seed");

        let page = list_catalog_for_state(&profile.id.to_string(), Some(25), None, &state)
            .await
            .expect("live catalog should load");
        let requests = handle.join().expect("mock registry thread should finish");

        assert!(!page.stale);
        assert_eq!(page.repositories.len(), 1);
        assert_eq!(page.repositories[0].repository_name, "alpine");
        assert_eq!(page.repositories[0].tag_count, 1);
        assert!(requests
            .iter()
            .any(|request| request.starts_with("GET /v2/empty/tags/list?n=100 HTTP/1.1")));

        let cached = list_repository_cache(&state.pool, profile.id)
            .await
            .expect("repository cache should load");
        assert_eq!(cached.len(), 1);
        assert_eq!(cached[0].repository_name, "alpine");
    }

    #[tokio::test]
    async fn remote_destructive_targets_are_rejected() {
        let error = ensure_local_registry_target("https://registry.example.com")
            .await
            .expect_err("remote destructive target should be rejected");

        assert_eq!(error.code, "REMOTE_REGISTRY_NOT_ALLOWED");
    }

    fn profile_input(
        name: &str,
        registry_url: &str,
        credential_ref: Option<&str>,
    ) -> RegistryProfileInput {
        RegistryProfileInput {
            name: Some(name.to_string()),
            registry_url: registry_url.to_string(),
            credential_ref: credential_ref.map(str::to_string),
            container_id: None,
            container_name: None,
            config_path: None,
        }
    }

    fn spawn_catalog_with_tag_counts_mock() -> (String, thread::JoinHandle<Vec<String>>) {
        let listener = TcpListener::bind("127.0.0.1:0").expect("mock registry should bind");
        let base_url = format!("http://{}", listener.local_addr().expect("mock address"));

        let handle = thread::spawn(move || {
            let mut requests = Vec::new();
            for _ in 0..3 {
                let (mut stream, _) = listener.accept().expect("mock request should connect");
                let request = read_request(&mut stream);
                let body = if request.starts_with("GET /v2/_catalog") {
                    r#"{"repositories":["alpine","empty"]}"#
                } else if request.starts_with("GET /v2/alpine/tags/list") {
                    r#"{"name":"alpine","tags":["latest"]}"#
                } else if request.starts_with("GET /v2/empty/tags/list") {
                    r#"{"name":"empty","tags":[]}"#
                } else {
                    r#"{"errors":[{"code":"NOT_FOUND"}]}"#
                };
                write_json_response(&mut stream, body);
                requests.push(request);
            }
            requests
        });

        (base_url, handle)
    }

    fn read_request(stream: &mut TcpStream) -> String {
        let mut request_buffer = [0_u8; 2048];
        let bytes_read = stream
            .read(&mut request_buffer)
            .expect("mock request should read");
        String::from_utf8_lossy(&request_buffer[..bytes_read]).to_string()
    }

    fn write_json_response(stream: &mut TcpStream, body: &str) {
        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            body.len(),
            body
        );
        stream
            .write_all(response.as_bytes())
            .expect("mock response should write");
    }
}
