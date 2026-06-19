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
            id, name, registry_url, credential_key, created_at, updated_at,
            container_id, container_name, image, port_mapping, config_path,
            storage_mounts, selected_at
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        ON CONFLICT(id) DO UPDATE SET
            name = excluded.name,
            registry_url = excluded.registry_url,
            credential_key = excluded.credential_key,
            updated_at = excluded.updated_at,
            container_id = excluded.container_id,
            container_name = excluded.container_name,
            image = excluded.image,
            port_mapping = excluded.port_mapping,
            config_path = excluded.config_path,
            storage_mounts = excluded.storage_mounts
        "#,
    )
    .bind(profile.id.to_string())
    .bind(&profile.name)
    .bind(&profile.registry_url)
    .bind(&profile.credential_ref)
    .bind(profile.created_at.to_rfc3339())
    .bind(profile.updated_at.to_rfc3339())
    .bind(profile.container_id.as_deref().unwrap_or_default())
    .bind(profile.container_name.as_deref().unwrap_or_default())
    .bind("")
    .bind("")
    .bind(&profile.config_path)
    .bind("[]")
    .bind(profile.updated_at.to_rfc3339())
    .execute(pool)
    .await?;

    prune_duplicate_registry_profiles(pool, profile).await?;

    Ok(())
}

pub async fn list_registry_profiles(pool: &SqlitePool) -> Result<Vec<RegistryProfile>, StoreError> {
    let rows = sqlx::query("SELECT * FROM registry_profiles ORDER BY name ASC, registry_url ASC")
        .fetch_all(pool)
        .await?;

    dedupe_profiles_by_url(
        rows.into_iter()
            .map(row_to_profile)
            .collect::<Result<Vec<_>, _>>()?,
    )
}

pub async fn get_selected_registry_profile(
    pool: &SqlitePool,
) -> Result<Option<RegistryProfile>, StoreError> {
    let row = sqlx::query(
        r#"
        SELECT * FROM registry_profiles
        WHERE selected_at IS NOT NULL
        ORDER BY selected_at DESC, updated_at DESC
        LIMIT 1
        "#,
    )
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

pub async fn get_registry_profile_by_url(
    pool: &SqlitePool,
    registry_url: &str,
) -> Result<Option<RegistryProfile>, StoreError> {
    let row = sqlx::query(
        r#"
        SELECT * FROM registry_profiles
        WHERE RTRIM(registry_url, '/') = RTRIM(?, '/')
        ORDER BY updated_at DESC, selected_at DESC
        LIMIT 1
        "#,
    )
    .bind(registry_url)
    .fetch_optional(pool)
    .await?;

    row.map(row_to_profile).transpose()
}

async fn prune_duplicate_registry_profiles(
    pool: &SqlitePool,
    profile: &RegistryProfile,
) -> Result<(), StoreError> {
    sqlx::query(
        r#"
        DELETE FROM registry_profiles
        WHERE id != ? AND RTRIM(registry_url, '/') = RTRIM(?, '/')
        "#,
    )
    .bind(profile.id.to_string())
    .bind(&profile.registry_url)
    .execute(pool)
    .await?;

    Ok(())
}

fn dedupe_profiles_by_url(
    profiles: Vec<RegistryProfile>,
) -> Result<Vec<RegistryProfile>, StoreError> {
    let mut deduped = Vec::new();
    for profile in profiles {
        if deduped.iter().any(|existing: &RegistryProfile| {
            registry_url_key(&existing.registry_url) == registry_url_key(&profile.registry_url)
        }) {
            continue;
        }
        deduped.push(profile);
    }
    Ok(deduped)
}

fn registry_url_key(value: &str) -> &str {
    value.trim().trim_end_matches('/')
}

pub async fn select_registry_profile(
    pool: &SqlitePool,
    id: Uuid,
    selected_at: DateTime<Utc>,
) -> Result<(), StoreError> {
    sqlx::query(
        r#"
        UPDATE registry_profiles
        SET selected_at = ?, updated_at = ?
        WHERE id = ?
        "#,
    )
    .bind(selected_at.to_rfc3339())
    .bind(selected_at.to_rfc3339())
    .bind(id.to_string())
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn delete_registry_profile(pool: &SqlitePool, id: Uuid) -> Result<bool, StoreError> {
    let result = sqlx::query("DELETE FROM registry_profiles WHERE id = ?")
        .bind(id.to_string())
        .execute(pool)
        .await?;

    Ok(result.rows_affected() > 0)
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
        "SELECT * FROM repository_cache WHERE registry_id = ? AND tag_count > 0 ORDER BY repository_name ASC",
    )
    .bind(registry_id.to_string())
    .fetch_all(pool)
    .await?;

    rows.into_iter().map(row_to_repository_cache).collect()
}

pub async fn delete_repository_cache(
    pool: &SqlitePool,
    registry_id: Uuid,
    repository_name: &str,
) -> Result<bool, StoreError> {
    let result =
        sqlx::query("DELETE FROM repository_cache WHERE registry_id = ? AND repository_name = ?")
            .bind(registry_id.to_string())
            .bind(repository_name)
            .execute(pool)
            .await?;

    Ok(result.rows_affected() > 0)
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
    let created_at: String = row.try_get("created_at")?;
    let updated_at: String = row.try_get("updated_at")?;
    let container_id: Option<String> = row.try_get("container_id")?;
    let container_name: Option<String> = row.try_get("container_name")?;

    Ok(RegistryProfile {
        id: Uuid::parse_str(row.try_get::<String, _>("id")?.as_str())?,
        name: row.try_get("name")?,
        registry_url: row.try_get("registry_url")?,
        credential_ref: row.try_get("credential_key")?,
        created_at: DateTime::parse_from_rfc3339(&created_at)?.with_timezone(&Utc),
        updated_at: DateTime::parse_from_rfc3339(&updated_at)?.with_timezone(&Utc),
        container_id: non_empty(container_id),
        container_name: non_empty(container_name),
        config_path: row.try_get("config_path")?,
    })
}

fn non_empty(value: Option<String>) -> Option<String> {
    value.and_then(|value| {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
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

#[cfg(test)]
mod profile_contract_tests {
    use super::{
        get_registry_profile, get_registry_profile_by_url, list_registry_profiles,
        save_registry_profile,
    };
    use crate::store::{migrate_database, RegistryProfile};
    use serde_json::{json, Value};
    use sqlx::{sqlite::SqlitePoolOptions, SqlitePool};
    use uuid::Uuid;

    const LOCAL_REGISTRY_URL: &str = "http://localhost:5000";
    const PROFILE_ID: &str = "6cfd850f-6283-43df-911b-493c29ec9867";
    const SELECTED_AT: &str = "2026-06-18T12:00:00Z";

    #[tokio::test]
    async fn migrates_legacy_container_profile_to_url_contract_without_status() {
        let pool = legacy_profile_pool().await;
        let profile_id = Uuid::parse_str(PROFILE_ID).expect("profile id should be valid");

        let profile = get_registry_profile(&pool, profile_id)
            .await
            .expect("legacy profile should load after migration")
            .expect("legacy profile should still exist after migration");
        let profile_json = serde_json::to_value(profile).expect("profile should serialize");

        assert_eq!(profile_json["id"], PROFILE_ID);
        assert_eq!(profile_json["name"], "registry");
        assert_eq!(profile_json["registryUrl"], LOCAL_REGISTRY_URL);
        assert_eq!(profile_json["containerId"], "legacy-container-id");
        assert_eq!(profile_json["containerName"], "registry");
        assert_required_timestamp(&profile_json, "createdAt");
        assert_required_timestamp(&profile_json, "updatedAt");
        assert_profile_omits_status_fields(&profile_json);
    }

    #[tokio::test]
    async fn saves_and_loads_url_only_profile_without_persisted_status() {
        let pool = empty_profile_pool().await;
        let profile_id = Uuid::parse_str(PROFILE_ID).expect("profile id should be valid");
        let profile: RegistryProfile = serde_json::from_value(json!({
            "id": PROFILE_ID,
            "name": "Local Registry",
            "registryUrl": LOCAL_REGISTRY_URL,
            "credentialRef": null,
            "createdAt": SELECTED_AT,
            "updatedAt": SELECTED_AT
        }))
        .expect("URL-only profile contract should deserialize without container metadata");

        save_registry_profile(&pool, &profile)
            .await
            .expect("URL-only profile should save");

        let loaded = get_registry_profile(&pool, profile_id)
            .await
            .expect("URL-only profile should load")
            .expect("URL-only profile should exist after save");
        let profile_json = serde_json::to_value(loaded).expect("profile should serialize");

        assert_eq!(profile_json["id"], PROFILE_ID);
        assert_eq!(profile_json["name"], "Local Registry");
        assert_eq!(profile_json["registryUrl"], LOCAL_REGISTRY_URL);
        assert_eq!(profile_json["createdAt"], SELECTED_AT);
        assert_eq!(profile_json["updatedAt"], SELECTED_AT);
        assert_url_only_profile_contract(&profile_json);
    }

    #[tokio::test]
    async fn saves_and_exposes_optional_container_association() {
        let pool = empty_profile_pool().await;
        let profile: RegistryProfile = serde_json::from_value(json!({
            "id": PROFILE_ID,
            "name": "Local Registry",
            "registryUrl": LOCAL_REGISTRY_URL,
            "credentialRef": null,
            "createdAt": SELECTED_AT,
            "updatedAt": SELECTED_AT,
            "containerId": "container-123",
            "containerName": "registry"
        }))
        .expect("container-linked profile contract should deserialize");

        save_registry_profile(&pool, &profile)
            .await
            .expect("container-linked profile should save");

        let profile_id = Uuid::parse_str(PROFILE_ID).expect("profile id should be valid");
        let loaded = get_registry_profile(&pool, profile_id)
            .await
            .expect("container-linked profile should load")
            .expect("container-linked profile should exist after save");
        let profile_json = serde_json::to_value(loaded).expect("profile should serialize");

        assert_eq!(profile_json["containerId"], "container-123");
        assert_eq!(profile_json["containerName"], "registry");
        assert_profile_omits_status_fields(&profile_json);
    }

    #[tokio::test]
    async fn saving_profile_prunes_duplicate_registry_urls() {
        let pool = empty_profile_pool().await;
        let original: RegistryProfile = serde_json::from_value(json!({
            "id": PROFILE_ID,
            "name": "Original",
            "registryUrl": "http://localhost:5000/",
            "credentialRef": null,
            "createdAt": SELECTED_AT,
            "updatedAt": SELECTED_AT
        }))
        .expect("original profile should deserialize");
        let replacement_id = Uuid::new_v4();
        let replacement: RegistryProfile = serde_json::from_value(json!({
            "id": replacement_id.to_string(),
            "name": "Replacement",
            "registryUrl": LOCAL_REGISTRY_URL,
            "credentialRef": null,
            "createdAt": SELECTED_AT,
            "updatedAt": "2026-06-18T12:30:00Z"
        }))
        .expect("replacement profile should deserialize");

        save_registry_profile(&pool, &original)
            .await
            .expect("original profile should save");
        save_registry_profile(&pool, &replacement)
            .await
            .expect("replacement profile should save and prune duplicate URL");

        let profiles = list_registry_profiles(&pool)
            .await
            .expect("profiles should load");
        assert_eq!(profiles.len(), 1);
        assert_eq!(profiles[0].id, replacement_id);

        let loaded = get_registry_profile_by_url(&pool, "http://localhost:5000/")
            .await
            .expect("profile should load by normalized URL")
            .expect("deduped profile should exist");
        assert_eq!(loaded.id, replacement_id);
    }

    async fn legacy_profile_pool() -> SqlitePool {
        let pool = empty_raw_pool().await;

        sqlx::query(
            r#"
            CREATE TABLE registry_profiles (
                id TEXT PRIMARY KEY NOT NULL,
                container_id TEXT NOT NULL,
                container_name TEXT NOT NULL,
                image TEXT NOT NULL,
                registry_url TEXT NOT NULL,
                port_mapping TEXT NOT NULL,
                config_path TEXT,
                storage_mounts TEXT NOT NULL,
                selected_at TEXT NOT NULL,
                last_health_check_at TEXT
            )
            "#,
        )
        .execute(&pool)
        .await
        .expect("legacy registry profile table should be created");

        sqlx::query(
            r#"
            INSERT INTO registry_profiles (
                id, container_id, container_name, image, registry_url, port_mapping,
                config_path, storage_mounts, selected_at, last_health_check_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(PROFILE_ID)
        .bind("legacy-container-id")
        .bind("registry")
        .bind("registry:2")
        .bind(LOCAL_REGISTRY_URL)
        .bind("0.0.0.0:5000->5000/tcp")
        .bind("/etc/docker/registry/config.yml")
        .bind("/tmp/registry:/var/lib/registry")
        .bind(SELECTED_AT)
        .bind("2026-06-18T12:15:00Z")
        .execute(&pool)
        .await
        .expect("legacy registry profile should be seeded");

        migrate_database(&pool)
            .await
            .expect("legacy profile schema should migrate");
        pool
    }

    async fn empty_profile_pool() -> SqlitePool {
        let pool = empty_raw_pool().await;
        migrate_database(&pool)
            .await
            .expect("profile schema should migrate");
        pool
    }

    async fn empty_raw_pool() -> SqlitePool {
        SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .expect("in-memory SQLite pool should open")
    }

    fn assert_required_timestamp(profile_json: &Value, field: &str) {
        assert!(
            profile_json.get(field).and_then(Value::as_str).is_some(),
            "profile should expose {field}: {profile_json:#}"
        );
    }

    fn assert_url_only_profile_contract(profile_json: &Value) {
        assert!(profile_json.get("containerId").is_none());
        assert!(profile_json.get("containerName").is_none());
        assert_profile_omits_status_fields(profile_json);
    }

    fn assert_profile_omits_status_fields(profile_json: &Value) {
        for forbidden_field in [
            "image",
            "portMapping",
            "storageMounts",
            "selectedAt",
            "lastHealthCheckAt",
            "healthStatus",
            "status",
        ] {
            assert!(
                profile_json.get(forbidden_field).is_none(),
                "profile should not expose persisted status field `{forbidden_field}`: {profile_json:#}"
            );
        }
    }
}
