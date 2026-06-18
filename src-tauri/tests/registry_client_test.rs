use std::io::{Read, Write};
use std::net::TcpListener;
use std::thread;

use registry_manager_lib::registry::{Manifest, RegistryClient, RegistryError};

const FIXTURE_URL: &str = "http://localhost:5001";
const FIXTURE_REPOSITORY: &str = "alpine";
const FIXTURE_TAG: &str = "latest";

#[tokio::test]
async fn registry_client_ping_returns_ok() {
    let client = RegistryClient::new(FIXTURE_URL.to_string());

    client
        .ping()
        .await
        .expect("registry fixture must respond to /v2/");
}

#[tokio::test]
async fn registry_client_lists_catalog_from_fixture() {
    let client = RegistryClient::new(FIXTURE_URL.to_string());

    let catalog = client
        .list_catalog(Some(100), None)
        .await
        .expect("catalog request should succeed");

    assert!(
        catalog
            .repositories
            .iter()
            .any(|repository| repository == FIXTURE_REPOSITORY),
        "expected seeded repository {FIXTURE_REPOSITORY:?}, got {:?}",
        catalog.repositories
    );
}

#[tokio::test]
async fn registry_client_lists_tags_with_pagination() {
    let client = RegistryClient::new(FIXTURE_URL.to_string());

    let tags = client
        .list_tags(FIXTURE_REPOSITORY, Some(1), None)
        .await
        .expect("tags request should succeed");

    assert_eq!(tags.name, FIXTURE_REPOSITORY);
    assert!(
        tags.tags.iter().any(|tag| tag == FIXTURE_TAG),
        "expected seeded tag {FIXTURE_TAG:?}, got {:?}",
        tags.tags
    );
}

#[tokio::test]
async fn registry_client_resolves_manifest_digest_from_header() {
    let client = RegistryClient::new(FIXTURE_URL.to_string());

    let digest = client
        .resolve_digest(FIXTURE_REPOSITORY, FIXTURE_TAG)
        .await
        .expect("digest resolution should succeed");

    assert!(
        digest.starts_with("sha256:"),
        "expected sha256 digest, got {digest}"
    );
}

#[tokio::test]
async fn registry_client_fetches_supported_manifest_media_type() {
    let client = RegistryClient::new(FIXTURE_URL.to_string());

    let manifest = client
        .fetch_manifest(FIXTURE_REPOSITORY, FIXTURE_TAG)
        .await
        .expect("manifest request should succeed");

    assert!(
        matches!(
            manifest,
            Manifest::DockerSchema2V2 { .. }
                | Manifest::DockerManifestList { .. }
                | Manifest::OciImageManifest { .. }
                | Manifest::OciImageIndex { .. }
        ),
        "expected recognized manifest variant, got {manifest:?}"
    );
}

#[tokio::test]
async fn registry_client_returns_typed_unsupported_media_type_error() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("mock server should bind");
    let base_url = format!("http://{}", listener.local_addr().expect("mock address"));

    let handle = thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("mock request should connect");
        let mut request_buffer = [0_u8; 2048];
        let _ = stream.read(&mut request_buffer);

        let body = br#"{"schemaVersion":2}"#;
        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/vnd.example.unsupported+json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            body.len()
        );
        stream
            .write_all(response.as_bytes())
            .expect("mock headers should write");
        stream.write_all(body).expect("mock body should write");
    });

    let client = RegistryClient::new(base_url);
    let error = client
        .fetch_manifest("alpine", "latest")
        .await
        .expect_err("unsupported content type should be typed error");

    handle.join().expect("mock server thread should finish");

    assert!(
        matches!(
            error,
            RegistryError::UnsupportedMediaType(ref content_type)
                if content_type == "application/vnd.example.unsupported+json"
        ),
        "expected UnsupportedMediaType, got {error:?}"
    );
}

#[tokio::test]
async fn registry_client_sends_basic_auth_when_configured() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("mock server should bind");
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

    let client = RegistryClient::new(base_url).with_basic_auth("user", "secret");
    client.ping().await.expect("authenticated ping should pass");

    let request = handle.join().expect("mock server thread should finish");
    assert!(
        request.lines().any(|line| line
            .to_ascii_lowercase()
            .starts_with("authorization: basic ")
            && line.trim_end().ends_with("dXNlcjpzZWNyZXQ=")),
        "expected Basic auth header"
    );
}
