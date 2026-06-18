use chrono::Utc;
use registry_manager_lib::commands::docker::{discover_registry_containers, get_docker_status};
use registry_manager_lib::registry::RegistryClient;
use registry_manager_lib::store::{
    connect_database, get_selected_registry_profile, list_manifest_cache, list_repository_cache,
    save_registry_profile, upsert_manifest_cache, upsert_repository_cache, ManifestCache,
    RegistryProfile, RepositoryCache,
};
use uuid::Uuid;

const USER_REGISTRY_URL: &str = "http://localhost:5000";

#[tokio::test]
async fn commands_discover_existing_localhost_5000_registry() {
    let status = get_docker_status()
        .await
        .expect("docker status command should not fail");
    assert!(
        status.reachable,
        "Docker daemon should be reachable: {status:?}"
    );

    let containers = discover_registry_containers()
        .await
        .expect("registry discovery command should succeed");
    assert!(
        containers
            .iter()
            .any(|container| container.registry_url.as_deref() == Some(USER_REGISTRY_URL)),
        "expected existing user registry on localhost:5000, got {containers:?}"
    );
}

#[tokio::test]
async fn registry_profile_and_cache_operations_round_trip_for_localhost_5000() {
    let pool = connect_database(std::path::Path::new(":memory:"))
        .await
        .expect("in-memory database should migrate");
    let profile = RegistryProfile {
        id: Uuid::new_v4(),
        container_id: "f8912fd523f0".to_string(),
        container_name: "registry".to_string(),
        image: "registry:2".to_string(),
        registry_url: USER_REGISTRY_URL.to_string(),
        port_mapping: "5000:5000".to_string(),
        config_path: None,
        storage_mounts: "[]".to_string(),
        selected_at: Utc::now(),
        last_health_check_at: None,
    };

    save_registry_profile(&pool, &profile)
        .await
        .expect("profile should persist");
    assert_eq!(
        get_selected_registry_profile(&pool)
            .await
            .expect("selected profile should load")
            .expect("selected profile should exist")
            .registry_url,
        USER_REGISTRY_URL
    );

    let client = RegistryClient::new(USER_REGISTRY_URL.to_string());
    client
        .ping()
        .await
        .expect("existing localhost:5000 registry should respond to /v2/");

    upsert_repository_cache(
        &pool,
        &RepositoryCache {
            registry_id: profile.id,
            repository_name: "alpine".to_string(),
            tag_count: 1,
            last_synced_at: Some(Utc::now()),
            sync_status: "fresh".to_string(),
        },
    )
    .await
    .expect("repository cache should persist");
    upsert_manifest_cache(
        &pool,
        &ManifestCache {
            registry_id: profile.id,
            repository_name: "alpine".to_string(),
            tag: "latest".to_string(),
            digest: "sha256:test".to_string(),
            media_type: "application/vnd.docker.distribution.manifest.v2+json".to_string(),
            platform_summary: Some("linux/arm64".to_string()),
            raw_json: "{}".to_string(),
            last_synced_at: Utc::now(),
            gc_status: None,
        },
    )
    .await
    .expect("manifest cache should persist");

    assert_eq!(
        list_repository_cache(&pool, profile.id)
            .await
            .unwrap()
            .len(),
        1
    );
    assert_eq!(
        list_manifest_cache(&pool, profile.id, "alpine")
            .await
            .unwrap()
            .len(),
        1
    );
}
