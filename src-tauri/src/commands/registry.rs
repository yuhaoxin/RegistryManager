use std::net::IpAddr;
use std::time::Duration;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tauri::State;
use tokio::time::timeout;
use url::Url;
use uuid::Uuid;

use crate::credentials::{CredentialStore, RegistryCredential, SystemKeyring};
use crate::docker::{discover_registry_containers, DockerClient, RegistryContainerSummary};
use crate::registry::{LayerSummary, Manifest, PlatformSummary, RegistryClient, RegistryError};
use crate::store::{
    get_registry_profile as load_registry_profile, get_selected_registry_profile as load_selected,
    list_manifest_cache, list_repository_cache, save_registry_profile,
    update_registry_health_check, upsert_manifest_cache, upsert_repository_cache, ManifestCache,
    RegistryProfile, RepositoryCache,
};

use super::{AppError, AppState};

const REQUEST_TIMEOUT: Duration = Duration::from_secs(10);
const REFRESH_TIMEOUT: Duration = Duration::from_secs(30);
const PAGE_SIZE: u32 = 25;
const TAG_COUNT_SCAN_LIMIT: u32 = 100;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RegistryProfileInput {
    pub container_id: String,
    pub container_name: String,
    pub image: String,
    pub registry_url: String,
    pub port_mapping: String,
    pub config_path: Option<String>,
    pub storage_mounts: String,
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
pub async fn select_registry_profile(
    profile: RegistryProfileInput,
    state: State<'_, AppState>,
) -> Result<RegistryProfile, AppError> {
    ensure_local_registry_target(&profile.registry_url).await?;

    let profile = RegistryProfile {
        id: Uuid::new_v4(),
        container_id: profile.container_id,
        container_name: profile.container_name,
        image: profile.image,
        registry_url: profile.registry_url,
        port_mapping: profile.port_mapping,
        config_path: profile.config_path,
        storage_mounts: profile.storage_mounts,
        selected_at: Utc::now(),
        last_health_check_at: None,
    };

    save_registry_profile(&state.pool, &profile).await?;
    Ok(profile)
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
        &profile.id.to_string(),
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
    SystemKeyring.delete(&profile.id.to_string())?;
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
    let profile = require_profile(&state, &profile_id).await?;
    let checked_at = Utc::now();

    let docker = match DockerClient::connect_local().await {
        Ok(client) => client,
        Err(error) => {
            return Ok(RegistryHealth {
                reachable: false,
                status: "docker_unavailable".to_string(),
                message: error.to_string(),
                checked_at,
            })
        }
    };

    match find_container(&docker, &profile.container_id).await {
        Ok(Some(container)) if container.state.as_deref() != Some("running") => {
            return Ok(RegistryHealth {
                reachable: false,
                status: "container_stopped".to_string(),
                message: format!(
                    "Container {} is {}.",
                    container.name,
                    container.state.unwrap_or_else(|| "not running".to_string())
                ),
                checked_at,
            })
        }
        Ok(Some(_)) => {}
        Ok(None) => {
            return Ok(RegistryHealth {
                reachable: false,
                status: "container_not_found".to_string(),
                message: "Selected registry container is no longer available.".to_string(),
                checked_at,
            })
        }
        Err(error) => return Err(error),
    }

    ensure_local_registry_target(&profile.registry_url).await?;
    let client = registry_client_for_profile(&profile)?;
    match timeout(REQUEST_TIMEOUT, client.ping()).await {
        Ok(Ok(())) => {
            update_registry_health_check(&state.pool, profile.id, checked_at).await?;
            Ok(RegistryHealth {
                reachable: true,
                status: "ok".to_string(),
                message: "/v2/ responded successfully.".to_string(),
                checked_at,
            })
        }
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
    let profile = require_profile(&state, &profile_id).await?;
    ensure_local_registry_target(&profile.registry_url).await?;
    let client = registry_client_for_profile(&profile)?;
    let page_size = n.unwrap_or(PAGE_SIZE).min(PAGE_SIZE);

    match timeout(REQUEST_TIMEOUT, client.list_catalog(Some(page_size), last)).await {
        Ok(Ok(catalog)) => {
            let synced_at = Utc::now();
            let mut repositories = Vec::with_capacity(catalog.repositories.len());
            for repository_name in catalog.repositories {
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
                upsert_repository_cache(&state.pool, &cache).await?;
                repositories.push(cache);
            }
            let next_last = next_cursor(&repositories, page_size, |repo| &repo.repository_name);

            Ok(CatalogPage {
                repositories,
                next_last,
                stale: false,
                last_synced_at: Some(synced_at),
                error: None,
            })
        }
        Ok(Err(error)) => cached_catalog_page(&state, profile.id, error.to_string()).await,
        Err(_) => {
            cached_catalog_page(&state, profile.id, "catalog request timed out".to_string()).await
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
    ensure_local_registry_target(&profile.registry_url).await?;
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
                sync_status: "fresh".to_string(),
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
    ensure_local_registry_target(&profile.registry_url).await?;
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

async fn require_profile(state: &AppState, profile_id: &str) -> Result<RegistryProfile, AppError> {
    let id = Uuid::parse_str(profile_id)?;
    load_registry_profile(&state.pool, id)
        .await?
        .ok_or_else(|| {
            AppError::new(
                "profile_not_found",
                "Selected registry profile was not found.",
            )
        })
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

    if matches_discovered_registry_binding(&url).await {
        return Ok(());
    }

    Err(remote_registry_not_allowed(registry_url))
}

pub(crate) fn registry_client_for_profile(
    profile: &RegistryProfile,
) -> Result<RegistryClient, AppError> {
    let client = RegistryClient::new(profile.registry_url.clone());
    let Some(credential) = SystemKeyring.load(&profile.id.to_string())? else {
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

async fn matches_discovered_registry_binding(url: &Url) -> bool {
    let Some(host) = url.host_str() else {
        return false;
    };
    let Some(port) = url.port_or_known_default() else {
        return false;
    };
    let Ok(docker) = DockerClient::connect_local().await else {
        return false;
    };
    let Ok(containers) = discover_registry_containers(&docker).await else {
        return false;
    };

    containers.iter().any(|container| {
        container
            .registry_url
            .as_deref()
            .is_some_and(|registry_url| registry_url_matches(registry_url, host, port))
            || container.ports.iter().any(|binding| {
                binding.host_port == Some(port)
                    && binding
                        .host_ip
                        .as_deref()
                        .is_some_and(|binding_host| hosts_equal(binding_host, host))
            })
    })
}

fn registry_url_matches(registry_url: &str, host: &str, port: u16) -> bool {
    Url::parse(registry_url)
        .ok()
        .and_then(|url| {
            Some((
                url.host_str().map(str::to_string)?,
                url.port_or_known_default()?,
            ))
        })
        .is_some_and(|(registry_host, registry_port)| {
            registry_port == port && hosts_equal(&registry_host, host)
        })
}

fn hosts_equal(left: &str, right: &str) -> bool {
    match (left.parse::<IpAddr>(), right.parse::<IpAddr>()) {
        (Ok(left), Ok(right)) => left == right,
        _ => left.eq_ignore_ascii_case(right),
    }
}

fn remote_registry_not_allowed(registry_url: &str) -> AppError {
    AppError::with_details(
        "REMOTE_REGISTRY_NOT_ALLOWED",
        "Only local Docker registry targets are allowed.",
        format!("Registry URL is not local or discovered from local Docker: {registry_url}"),
    )
}

async fn find_container(
    client: &DockerClient,
    container_id: &str,
) -> Result<Option<RegistryContainerSummary>, AppError> {
    let containers = discover_registry_containers(client).await?;
    Ok(containers
        .into_iter()
        .find(|container| container.id == container_id || container.id.starts_with(container_id)))
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
    ensure_local_registry_target(&profile.registry_url).await?;
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
        Ok(Ok(tags)) => Ok((tags.tags.len() as i64, "fresh")),
        Ok(Err(_)) | Err(_) => Ok((
            list_manifest_cache(pool, profile_id, repository)
                .await?
                .len() as i64,
            "tag_count_stale",
        )),
    }
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
    use super::ensure_local_registry_target;

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
}
