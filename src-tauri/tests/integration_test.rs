use registry_manager_lib::commands::docker::get_docker_status;

#[tokio::test]
async fn get_docker_status_returns_structured_status() {
    let expected_context =
        std::env::var("DOCKER_CONTEXT").unwrap_or_else(|_| "default".to_string());
    let status = get_docker_status()
        .await
        .expect("docker status command should report availability without failing");

    assert_eq!(status.context, expected_context);
    if status.reachable {
        assert!(status.error.is_none());
    } else {
        assert!(status.error.is_some());
    }
}
