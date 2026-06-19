use std::path::{Path, PathBuf};

use chrono::Utc;
use sqlx::{sqlite::SqlitePoolOptions, Row, SqlitePool};

use super::StoreError;

const SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS registry_profiles (
    id TEXT PRIMARY KEY NOT NULL,
    name TEXT NOT NULL,
    registry_url TEXT NOT NULL,
    credential_key TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    container_id TEXT,
    container_name TEXT,
    image TEXT,
    port_mapping TEXT,
    config_path TEXT,
    storage_mounts TEXT,
    selected_at TEXT
);

CREATE TABLE IF NOT EXISTS repository_cache (
    registry_id TEXT NOT NULL,
    repository_name TEXT NOT NULL,
    tag_count INTEGER NOT NULL DEFAULT 0,
    last_synced_at TEXT,
    sync_status TEXT NOT NULL,
    PRIMARY KEY (registry_id, repository_name),
    FOREIGN KEY (registry_id) REFERENCES registry_profiles(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS manifest_cache (
    registry_id TEXT NOT NULL,
    repository_name TEXT NOT NULL,
    tag TEXT NOT NULL,
    digest TEXT NOT NULL,
    media_type TEXT NOT NULL,
    platform_summary TEXT,
    raw_json TEXT NOT NULL,
    last_synced_at TEXT NOT NULL,
    gc_status TEXT,
    PRIMARY KEY (registry_id, repository_name, tag),
    FOREIGN KEY (registry_id) REFERENCES registry_profiles(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS audit_events (
    id TEXT PRIMARY KEY NOT NULL,
    timestamp TEXT NOT NULL,
    action TEXT NOT NULL,
    registry_id TEXT,
    container_id TEXT,
    repository_name TEXT,
    tag TEXT,
    digest TEXT,
    status TEXT NOT NULL,
    duration_ms INTEGER,
    error_message TEXT,
    log_excerpt TEXT
);

CREATE TABLE IF NOT EXISTS gc_transactions (
    id TEXT PRIMARY KEY NOT NULL,
    registry_id TEXT NOT NULL,
    container_id TEXT NOT NULL,
    original_state TEXT,
    original_image TEXT NOT NULL,
    mount_summary TEXT NOT NULL,
    config_path TEXT,
    started_at TEXT NOT NULL,
    finished_at TEXT,
    exit_code INTEGER,
    log_path TEXT,
    recovery_action TEXT,
    final_health_status TEXT,
    FOREIGN KEY (registry_id) REFERENCES registry_profiles(id) ON DELETE CASCADE
);
"#;

pub async fn connect_app_database() -> Result<SqlitePool, StoreError> {
    let mut path = dirs::data_local_dir().ok_or(StoreError::AppDataDirUnavailable)?;
    path.push("registry-manager");
    std::fs::create_dir_all(&path)?;
    path.push("registry-manager.sqlite");
    connect_database(&path).await
}

pub async fn connect_database(path: &Path) -> Result<SqlitePool, StoreError> {
    let database_url = if path == Path::new(":memory:") {
        "sqlite::memory:".to_string()
    } else {
        ensure_parent_dir(path)?;
        format!("sqlite://{}?mode=rwc", path.display())
    };

    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await?;
    migrate_database(&pool).await?;
    Ok(pool)
}

pub async fn migrate_database(pool: &SqlitePool) -> Result<(), StoreError> {
    sqlx::query("PRAGMA foreign_keys = ON;")
        .execute(pool)
        .await?;
    for statement in SCHEMA
        .split(';')
        .map(str::trim)
        .filter(|statement| !statement.is_empty())
    {
        sqlx::query(statement).execute(pool).await?;
    }
    ensure_registry_profile_columns(pool).await?;
    ensure_manifest_optional_columns(pool).await?;
    Ok(())
}

async fn ensure_registry_profile_columns(pool: &SqlitePool) -> Result<(), StoreError> {
    for (name, definition) in [
        ("name", "TEXT"),
        ("credential_key", "TEXT"),
        ("created_at", "TEXT"),
        ("updated_at", "TEXT"),
        ("container_id", "TEXT"),
        ("container_name", "TEXT"),
        ("image", "TEXT"),
        ("port_mapping", "TEXT"),
        ("config_path", "TEXT"),
        ("storage_mounts", "TEXT"),
        ("selected_at", "TEXT"),
    ] {
        add_column_if_missing(pool, "registry_profiles", name, definition).await?;
    }

    let now = Utc::now().to_rfc3339();
    sqlx::query(
        r#"
        UPDATE registry_profiles
        SET
            name = COALESCE(NULLIF(TRIM(name), ''), NULLIF(TRIM(container_name), ''), registry_url),
            created_at = COALESCE(created_at, selected_at, ?),
            updated_at = COALESCE(updated_at, selected_at, created_at, ?)
        WHERE name IS NULL
            OR TRIM(name) = ''
            OR created_at IS NULL
            OR updated_at IS NULL
        "#,
    )
    .bind(&now)
    .bind(&now)
    .execute(pool)
    .await?;

    Ok(())
}

async fn ensure_manifest_optional_columns(pool: &SqlitePool) -> Result<(), StoreError> {
    add_column_if_missing(pool, "manifest_cache", "gc_status", "TEXT").await?;
    Ok(())
}

async fn add_column_if_missing(
    pool: &SqlitePool,
    table: &str,
    name: &str,
    definition: &str,
) -> Result<(), StoreError> {
    if !table_has_column(pool, table, name).await? {
        let statement = format!("ALTER TABLE {table} ADD COLUMN {name} {definition}");
        sqlx::query(&statement).execute(pool).await?;
    }
    Ok(())
}

async fn table_has_column(pool: &SqlitePool, table: &str, name: &str) -> Result<bool, StoreError> {
    let statement = format!("PRAGMA table_info({table})");
    let columns = sqlx::query(&statement).fetch_all(pool).await?;
    Ok(columns
        .iter()
        .any(|row| row.try_get::<String, _>("name").ok().as_deref() == Some(name)))
}

fn ensure_parent_dir(path: &Path) -> Result<(), StoreError> {
    let parent: Option<PathBuf> = path.parent().map(Path::to_path_buf);
    if let Some(parent) = parent {
        std::fs::create_dir_all(parent)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use sqlx::{sqlite::SqlitePoolOptions, Row};
    use uuid::Uuid;

    use super::migrate_database;

    #[tokio::test]
    async fn profile_migration_preserves_legacy_registry_url_and_omits_status_fields() {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .expect("in-memory database should connect");

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
        .expect("legacy registry_profiles table should be created");

        let profile_id = Uuid::new_v4();
        let selected_at = "2026-06-18T12:00:00Z";
        sqlx::query(
            r#"
            INSERT INTO registry_profiles (
                id, container_id, container_name, image, registry_url, port_mapping,
                config_path, storage_mounts, selected_at, last_health_check_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(profile_id.to_string())
        .bind("legacy-container")
        .bind("legacy-registry")
        .bind("registry:2")
        .bind("http://localhost:5000")
        .bind("5000:5000")
        .bind(Option::<String>::None)
        .bind("[]")
        .bind(selected_at)
        .bind("2026-06-18T12:05:00Z")
        .execute(&pool)
        .await
        .expect("legacy profile should be inserted");

        migrate_database(&pool)
            .await
            .expect("legacy database should migrate");

        let columns = sqlx::query("PRAGMA table_info(registry_profiles)")
            .fetch_all(&pool)
            .await
            .expect("registry profile columns should be readable");
        let column_names = columns
            .iter()
            .map(|row| row.try_get::<String, _>("name").expect("column name"))
            .collect::<Vec<_>>();
        for expected in [
            "id",
            "name",
            "registry_url",
            "credential_key",
            "created_at",
            "updated_at",
            "container_id",
            "container_name",
        ] {
            assert!(
                column_names.iter().any(|column| column == expected),
                "registry_profiles should include {expected}, got {column_names:?}"
            );
        }

        let profile = crate::store::get_selected_registry_profile(&pool)
            .await
            .expect("selected profile should load")
            .expect("selected legacy profile should exist");
        assert_eq!(profile.registry_url, "http://localhost:5000");

        let value = serde_json::to_value(&profile).expect("profile should serialize");
        assert_eq!(value["containerId"], "legacy-container");
        assert_eq!(value["containerName"], "legacy-registry");
        assert!(value.get("status").is_none());
        assert!(value.get("healthStatus").is_none());
        assert!(value.get("lastHealthCheckAt").is_none());
    }
}
