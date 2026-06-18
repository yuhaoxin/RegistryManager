use std::path::{Path, PathBuf};

use sqlx::{sqlite::SqlitePoolOptions, Row, SqlitePool};

use super::StoreError;

const SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS registry_profiles (
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
    ensure_optional_columns(pool).await?;
    Ok(())
}

async fn ensure_optional_columns(pool: &SqlitePool) -> Result<(), StoreError> {
    let columns = sqlx::query("PRAGMA table_info(manifest_cache)")
        .fetch_all(pool)
        .await?;
    if !columns
        .iter()
        .any(|row| row.try_get::<String, _>("name").ok().as_deref() == Some("gc_status"))
    {
        sqlx::query("ALTER TABLE manifest_cache ADD COLUMN gc_status TEXT")
            .execute(pool)
            .await?;
    }
    Ok(())
}

fn ensure_parent_dir(path: &Path) -> Result<(), StoreError> {
    let parent: Option<PathBuf> = path.parent().map(Path::to_path_buf);
    if let Some(parent) = parent {
        std::fs::create_dir_all(parent)?;
    }
    Ok(())
}
