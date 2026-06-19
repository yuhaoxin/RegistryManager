use chrono::Utc;
use registry_manager_lib::audit::{list_audit_events, log_audit_event, AuditAction, AuditEvent};
use registry_manager_lib::store::{
    connect_database, list_manifest_cache, save_registry_profile, update_manifest_gc_status,
    update_pending_gc_records, upsert_manifest_cache, ManifestCache, RegistryProfile,
};
use uuid::Uuid;

fn test_profile() -> RegistryProfile {
    let now = Utc::now();
    RegistryProfile {
        id: Uuid::new_v4(),
        name: "registry-test".to_string(),
        registry_url: "http://localhost:5001".to_string(),
        credential_ref: None,
        created_at: now,
        updated_at: now,
        container_id: Some("fixture-registry".to_string()),
        container_name: Some("registry-test".to_string()),
        config_path: None,
    }
}

#[tokio::test]
async fn delete_audit_and_pending_gc_records_round_trip() {
    let pool = connect_database(std::path::Path::new(":memory:"))
        .await
        .expect("schema should migrate");
    let profile = test_profile();
    save_registry_profile(&pool, &profile).await.unwrap();
    upsert_manifest_cache(
        &pool,
        &ManifestCache {
            registry_id: profile.id,
            repository_name: "alpine".to_string(),
            tag: "latest".to_string(),
            digest: "sha256:abc123def4567890".to_string(),
            media_type: "application/vnd.docker.distribution.manifest.v2+json".to_string(),
            platform_summary: None,
            raw_json: "{}".to_string(),
            last_synced_at: Utc::now(),
            gc_status: None,
        },
    )
    .await
    .unwrap();

    update_manifest_gc_status(
        &pool,
        profile.id,
        "alpine",
        "sha256:abc123def4567890",
        "pending_gc",
    )
    .await
    .unwrap();
    log_audit_event(
        &pool,
        &AuditEvent {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            action: AuditAction::DeleteManifest,
            registry_id: Some(profile.id),
            container_id: profile.container_id.clone(),
            repository_name: Some("alpine".to_string()),
            tag: Some("latest".to_string()),
            digest: Some("sha256:abc123def4567890".to_string()),
            status: "pending_gc".to_string(),
            duration_ms: Some(12),
            error_message: None,
            log_excerpt: Some("{\"registry\":\"http://localhost:5001\"}".to_string()),
        },
    )
    .await
    .unwrap();

    let records = list_manifest_cache(&pool, profile.id, "alpine")
        .await
        .unwrap();
    assert_eq!(records[0].gc_status.as_deref(), Some("pending_gc"));
    let events = list_audit_events(&pool, 10, 0).await.unwrap();
    assert_eq!(events[0].action, AuditAction::DeleteManifest);
    assert_eq!(events[0].status, "pending_gc");
}

#[tokio::test]
async fn gc_status_updates_pending_records_to_completed_or_failed() {
    let pool = connect_database(std::path::Path::new(":memory:"))
        .await
        .expect("schema should migrate");
    let profile = test_profile();
    save_registry_profile(&pool, &profile).await.unwrap();
    upsert_manifest_cache(
        &pool,
        &ManifestCache {
            registry_id: profile.id,
            repository_name: "busybox".to_string(),
            tag: "latest".to_string(),
            digest: "sha256:def4567890abc123".to_string(),
            media_type: "application/vnd.docker.distribution.manifest.v2+json".to_string(),
            platform_summary: None,
            raw_json: "{}".to_string(),
            last_synced_at: Utc::now(),
            gc_status: Some("pending_gc".to_string()),
        },
    )
    .await
    .unwrap();

    update_pending_gc_records(&pool, profile.id, "gc_completed")
        .await
        .unwrap();

    let records = list_manifest_cache(&pool, profile.id, "busybox")
        .await
        .unwrap();
    assert_eq!(records[0].gc_status.as_deref(), Some("gc_completed"));
}
