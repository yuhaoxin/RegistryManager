use std::path::Path;
use std::sync::LazyLock;

use tokio::sync::{Mutex, MutexGuard};

use registry_manager_lib::docker::{
    discover_registry_containers, verify_local_docker_context, DockerClient, DockerError,
};
use registry_manager_lib::store::connect_database;

static ENV_LOCK: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));

#[tokio::test]
async fn docker_discovery_finds_fixture_registry_on_port_5001() {
    let _guard = env_lock().await;
    let client = DockerClient::connect_local()
        .await
        .expect("local Docker daemon must be available for fixture discovery");

    let containers = discover_registry_containers(&client)
        .await
        .expect("registry discovery should succeed");

    let registry = containers
        .iter()
        .find(|container| {
            container
                .ports
                .iter()
                .any(|port| port.container_port == 5000 && port.host_port == Some(5001))
        })
        .expect("registry fixture should be published on localhost:5001");

    assert!(registry.image.contains("registry"));
    assert!(registry.name.contains("registry"));
    assert_eq!(
        registry.registry_url.as_deref(),
        Some("http://localhost:5001")
    );
    assert!(registry
        .mounts
        .iter()
        .any(|mount| mount.destination.as_deref() == Some("/var/lib/registry")));
}

#[tokio::test]
async fn docker_discovery_inspect_includes_required_container_fields() {
    let _guard = env_lock().await;
    let client = DockerClient::connect_local()
        .await
        .expect("local Docker daemon must be available for fixture discovery");

    let containers = discover_registry_containers(&client)
        .await
        .expect("registry discovery should succeed");

    let registry = containers
        .iter()
        .find(|container| container.registry_url.as_deref() == Some("http://localhost:5001"))
        .expect("registry fixture should be discovered");

    assert!(!registry.id.is_empty());
    assert!(!registry.image.is_empty());
    assert!(registry.state.is_some());
    assert!(registry
        .env
        .iter()
        .any(|entry| entry == "REGISTRY_STORAGE_DELETE_ENABLED=true"));
    assert!(registry.restart_policy.is_some());
}

#[tokio::test]
async fn docker_discovery_rejects_non_local_docker_context() {
    let _guard = env_lock().await;
    let previous = std::env::var("DOCKER_HOST").ok();
    std::env::set_var("DOCKER_HOST", "tcp://192.0.2.10:2375");

    let error = verify_local_docker_context().expect_err("remote DOCKER_HOST should be rejected");

    match previous {
        Some(value) => std::env::set_var("DOCKER_HOST", value),
        None => std::env::remove_var("DOCKER_HOST"),
    }

    assert!(matches!(error, DockerError::RemoteContext(_)));
}

#[tokio::test]
async fn docker_discovery_reports_unavailable_daemon() {
    let _guard = env_lock().await;
    let previous = std::env::var("DOCKER_HOST").ok();
    std::env::set_var(
        "DOCKER_HOST",
        "unix:///tmp/registry-manager-missing-docker.sock",
    );

    let error = DockerClient::connect_local()
        .await
        .expect_err("missing local socket should be unavailable");

    match previous {
        Some(value) => std::env::set_var("DOCKER_HOST", value),
        None => std::env::remove_var("DOCKER_HOST"),
    }

    assert!(matches!(error, DockerError::DockerUnavailable(_)));
}

#[tokio::test]
async fn docker_discovery_sqlite_migration_creates_schema_from_empty_database() {
    let pool = connect_database(Path::new(":memory:"))
        .await
        .expect("empty SQLite database should migrate");

    for table in [
        "registry_profiles",
        "repository_cache",
        "manifest_cache",
        "audit_events",
        "gc_transactions",
    ] {
        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = ?",
        )
        .bind(table)
        .fetch_one(&pool)
        .await
        .expect("schema lookup should succeed");
        assert_eq!(count, 1, "expected table {table} to exist");
    }
}

async fn env_lock() -> MutexGuard<'static, ()> {
    ENV_LOCK.lock().await
}
