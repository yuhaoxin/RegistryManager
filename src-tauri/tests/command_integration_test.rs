use chrono::Utc;
use registry_manager_lib::registry::RegistryClient;
use registry_manager_lib::store::{
    connect_database, get_selected_registry_profile, list_manifest_cache, list_repository_cache,
    save_registry_profile, upsert_manifest_cache, upsert_repository_cache, ManifestCache,
    RegistryProfile, RepositoryCache,
};
use std::io::{Read, Write};
use std::net::TcpListener;
use std::thread;
use uuid::Uuid;

#[tokio::test]
async fn registry_profile_and_cache_operations_round_trip_with_mock_registry() {
    let (registry_url, handle) = spawn_ping_registry_mock();
    let pool = connect_database(std::path::Path::new(":memory:"))
        .await
        .expect("in-memory database should migrate");
    let now = Utc::now();
    let profile = RegistryProfile {
        id: Uuid::new_v4(),
        name: "registry".to_string(),
        registry_url: registry_url.clone(),
        credential_ref: None,
        created_at: now,
        updated_at: now,
        container_id: Some("f8912fd523f0".to_string()),
        container_name: Some("registry".to_string()),
        config_path: None,
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
        registry_url
    );

    let client = RegistryClient::new(profile.registry_url.clone());
    client
        .ping()
        .await
        .expect("mock registry should respond to /v2/");
    let request = handle.join().expect("mock registry thread should finish");
    assert!(request.starts_with("GET /v2/ HTTP/1.1"), "got {request}");

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

fn spawn_ping_registry_mock() -> (String, thread::JoinHandle<String>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("mock registry should bind");
    let base_url = format!("http://{}", listener.local_addr().expect("mock address"));

    let handle = thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("mock request should connect");
        let mut request_buffer = [0_u8; 2048];
        let bytes_read = stream
            .read(&mut request_buffer)
            .expect("mock request should read");
        let request = String::from_utf8_lossy(&request_buffer[..bytes_read]).to_string();

        let response = "HTTP/1.1 200 OK\r\nContent-Length: 0\r\nConnection: close\r\n\r\n";
        stream
            .write_all(response.as_bytes())
            .expect("mock response should write");

        request
    });

    (base_url, handle)
}
