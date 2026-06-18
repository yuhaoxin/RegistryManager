use chrono::{DateTime, Utc};
use sqlx::{Row, SqlitePool};
use uuid::Uuid;

use super::{ManifestCache, RegistryProfile, RepositoryCache, StoreError};

pub async fn save_registry_profile(
    pool: &SqlitePool,
    profile: &RegistryProfile,
) -> Result<(), StoreError> {
    sqlx::query(
        r#"
        INSERT INTO registry_profiles (
            id, container_id, container_name, image, registry_url, port_mapping,
            config_path, storage_mounts, selected_at, last_health_check_at
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        ON CONFLICT(id) DO UPDATE SET
            container_id = excluded.container_id,
            container_name = excluded.container_name,
            image = excluded.image,
            registry_url = excluded.registry_url,
            port_mapping = excluded.port_mapping,
            config_path = excluded.config_path,
            storage_mounts = excluded.storage_mounts,
            selected_at = excluded.selected_at,
            last_health_check_at = excluded.last_health_check_at
        "#,
    )
    .bind(profile.id.to_string())
    .bind(&profile.container_id)
    .bind(&profile.container_name)
    .bind(&profile.image)
    .bind(&profile.registry_url)
    .bind(&profile.port_mapping)
    .bind(&profile.config_path)
    .bind(&profile.storage_mounts)
    .bind(profile.selected_at.to_rfc3339())
    .bind(profile.last_health_check_at.map(|value| value.to_rfc3339()))
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn get_selected_registry_profile(
    pool: &SqlitePool,
) -> Result<Option<RegistryProfile>, StoreError> {
    let row = sqlx::query("SELECT * FROM registry_profiles ORDER BY selected_at DESC LIMIT 1")
        .fetch_optional(pool)
        .await?;

    row.map(row_to_profile).transpose()
}

pub async fn get_registry_profile(
    pool: &SqlitePool,
    id: Uuid,
) -> Result<Option<RegistryProfile>, StoreError> {
    let row = sqlx::query("SELECT * FROM registry_profiles WHERE id = ?")
        .bind(id.to_string())
        .fetch_optional(pool)
        .await?;

    row.map(row_to_profile).transpose()
}

pub async fn update_registry_health_check(
    pool: &SqlitePool,
    id: Uuid,
    checked_at: DateTime<Utc>,
) -> Result<(), StoreError> {
    sqlx::query("UPDATE registry_profiles SET last_health_check_at = ? WHERE id = ?")
        .bind(checked_at.to_rfc3339())
        .bind(id.to_string())
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn upsert_repository_cache(
    pool: &SqlitePool,
    cache: &RepositoryCache,
) -> Result<(), StoreError> {
    sqlx::query(
        r#"
        INSERT INTO repository_cache (
            registry_id, repository_name, tag_count, last_synced_at, sync_status
        ) VALUES (?, ?, ?, ?, ?)
        ON CONFLICT(registry_id, repository_name) DO UPDATE SET
            tag_count = excluded.tag_count,
            last_synced_at = excluded.last_synced_at,
            sync_status = excluded.sync_status
        "#,
    )
    .bind(cache.registry_id.to_string())
    .bind(&cache.repository_name)
    .bind(cache.tag_count)
    .bind(cache.last_synced_at.map(|value| value.to_rfc3339()))
    .bind(&cache.sync_status)
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn list_repository_cache(
    pool: &SqlitePool,
    registry_id: Uuid,
) -> Result<Vec<RepositoryCache>, StoreError> {
    let rows = sqlx::query(
        "SELECT * FROM repository_cache WHERE registry_id = ? ORDER BY repository_name ASC",
    )
    .bind(registry_id.to_string())
    .fetch_all(pool)
    .await?;

    rows.into_iter().map(row_to_repository_cache).collect()
}

pub async fn upsert_manifest_cache(
    pool: &SqlitePool,
    cache: &ManifestCache,
) -> Result<(), StoreError> {
    sqlx::query(
        r#"
        INSERT INTO manifest_cache (
            registry_id, repository_name, tag, digest, media_type,
            platform_summary, raw_json, last_synced_at, gc_status
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
        ON CONFLICT(registry_id, repository_name, tag) DO UPDATE SET
            digest = excluded.digest,
            media_type = excluded.media_type,
            platform_summary = excluded.platform_summary,
            raw_json = excluded.raw_json,
            last_synced_at = excluded.last_synced_at,
            gc_status = excluded.gc_status
        "#,
    )
    .bind(cache.registry_id.to_string())
    .bind(&cache.repository_name)
    .bind(&cache.tag)
    .bind(&cache.digest)
    .bind(&cache.media_type)
    .bind(&cache.platform_summary)
    .bind(&cache.raw_json)
    .bind(cache.last_synced_at.to_rfc3339())
    .bind(&cache.gc_status)
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn list_manifest_cache(
    pool: &SqlitePool,
    registry_id: Uuid,
    repository: &str,
) -> Result<Vec<ManifestCache>, StoreError> {
    let rows = sqlx::query(
        "SELECT * FROM manifest_cache WHERE registry_id = ? AND repository_name = ? ORDER BY tag ASC",
    )
    .bind(registry_id.to_string())
    .bind(repository)
    .fetch_all(pool)
    .await?;

    rows.into_iter().map(row_to_manifest_cache).collect()
}

pub async fn list_manifest_cache_by_digest(
    pool: &SqlitePool,
    registry_id: Uuid,
    repository: &str,
    digest: &str,
) -> Result<Vec<ManifestCache>, StoreError> {
    let rows = sqlx::query(
        "SELECT * FROM manifest_cache WHERE registry_id = ? AND repository_name = ? AND digest = ? ORDER BY tag ASC",
    )
    .bind(registry_id.to_string())
    .bind(repository)
    .bind(digest)
    .fetch_all(pool)
    .await?;

    rows.into_iter().map(row_to_manifest_cache).collect()
}

pub async fn update_manifest_gc_status(
    pool: &SqlitePool,
    registry_id: Uuid,
    repository: &str,
    digest: &str,
    gc_status: &str,
) -> Result<(), StoreError> {
    sqlx::query(
        "UPDATE manifest_cache SET gc_status = ? WHERE registry_id = ? AND repository_name = ? AND digest = ?",
    )
    .bind(gc_status)
    .bind(registry_id.to_string())
    .bind(repository)
    .bind(digest)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn update_pending_gc_records(
    pool: &SqlitePool,
    registry_id: Uuid,
    gc_status: &str,
) -> Result<(), StoreError> {
    sqlx::query("UPDATE manifest_cache SET gc_status = ? WHERE registry_id = ? AND gc_status = 'pending_gc'")
        .bind(gc_status)
        .bind(registry_id.to_string())
        .execute(pool)
        .await?;
    Ok(())
}

fn row_to_profile(row: sqlx::sqlite::SqliteRow) -> Result<RegistryProfile, StoreError> {
    let selected_at: String = row.try_get("selected_at")?;
    let last_health_check_at: Option<String> = row.try_get("last_health_check_at")?;

    Ok(RegistryProfile {
        id: Uuid::parse_str(row.try_get::<String, _>("id")?.as_str())?,
        container_id: row.try_get("container_id")?,
        container_name: row.try_get("container_name")?,
        image: row.try_get("image")?,
        registry_url: row.try_get("registry_url")?,
        port_mapping: row.try_get("port_mapping")?,
        config_path: row.try_get("config_path")?,
        storage_mounts: row.try_get("storage_mounts")?,
        selected_at: DateTime::parse_from_rfc3339(&selected_at)?.with_timezone(&Utc),
        last_health_check_at: last_health_check_at
            .map(|value| DateTime::parse_from_rfc3339(&value).map(|date| date.with_timezone(&Utc)))
            .transpose()?,
    })
}

fn row_to_repository_cache(row: sqlx::sqlite::SqliteRow) -> Result<RepositoryCache, StoreError> {
    let last_synced_at: Option<String> = row.try_get("last_synced_at")?;

    Ok(RepositoryCache {
        registry_id: Uuid::parse_str(row.try_get::<String, _>("registry_id")?.as_str())?,
        repository_name: row.try_get("repository_name")?,
        tag_count: row.try_get("tag_count")?,
        last_synced_at: last_synced_at
            .map(|value| DateTime::parse_from_rfc3339(&value).map(|date| date.with_timezone(&Utc)))
            .transpose()?,
        sync_status: row.try_get("sync_status")?,
    })
}

fn row_to_manifest_cache(row: sqlx::sqlite::SqliteRow) -> Result<ManifestCache, StoreError> {
    let last_synced_at: String = row.try_get("last_synced_at")?;

    Ok(ManifestCache {
        registry_id: Uuid::parse_str(row.try_get::<String, _>("registry_id")?.as_str())?,
        repository_name: row.try_get("repository_name")?,
        tag: row.try_get("tag")?,
        digest: row.try_get("digest")?,
        media_type: row.try_get("media_type")?,
        platform_summary: row.try_get("platform_summary")?,
        raw_json: row.try_get("raw_json")?,
        last_synced_at: DateTime::parse_from_rfc3339(&last_synced_at)?.with_timezone(&Utc),
        gc_status: row.try_get("gc_status")?,
    })
}
