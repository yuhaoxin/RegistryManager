use tauri::State;
use uuid::Uuid;

use crate::store::{list_manifest_cache, list_repository_cache, ManifestCache, RepositoryCache};

use super::{AppError, AppState};

#[tauri::command]
pub async fn get_cached_repositories(
    profile_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<RepositoryCache>, AppError> {
    Ok(list_repository_cache(&state.pool, Uuid::parse_str(&profile_id)?).await?)
}

#[tauri::command]
pub async fn get_cached_tags(
    profile_id: String,
    repository: String,
    state: State<'_, AppState>,
) -> Result<Vec<ManifestCache>, AppError> {
    Ok(list_manifest_cache(&state.pool, Uuid::parse_str(&profile_id)?, &repository).await?)
}
