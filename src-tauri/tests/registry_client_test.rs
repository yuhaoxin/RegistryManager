use std::io::{Read, Write};
use std::net::TcpListener;
use std::thread;

use registry_manager_lib::registry::{Manifest, RegistryClient, RegistryError};

const FIXTURE_REPOSITORY: &str = "alpine";
const FIXTURE_TAG: &str = "latest";
const DOCKER_SCHEMA2_MANIFEST: &str = "application/vnd.docker.distribution.manifest.v2+json";

#[tokio::test]
async fn registry_client_ping_returns_ok() {
    let (base_url, handle) = spawn_registry_mock(MockResponse::empty_ok());
    let client = RegistryClient::new(base_url);

    client
        .ping()
        .await
        .expect("mock registry should respond to /v2/");

    let request = handle.join().expect("mock server thread should finish");
    assert!(request.starts_with("GET /v2/ HTTP/1.1"), "got {request}");
}

#[tokio::test]
async fn registry_client_lists_catalog_from_fixture() {
    let (base_url, handle) = spawn_registry_mock(MockResponse::json_ok(
        r#"{"repositories":["alpine","busybox"]}"#,
    ));
    let client = RegistryClient::new(base_url);

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

    let request = handle.join().expect("mock server thread should finish");
    assert!(
        request.starts_with("GET /v2/_catalog?n=100 HTTP/1.1"),
        "got {request}"
    );
}

#[tokio::test]
async fn registry_client_lists_tags_with_pagination() {
    let (base_url, handle) = spawn_registry_mock(MockResponse::json_ok(
        r#"{"name":"alpine","tags":["latest","3.20"]}"#,
    ));
    let client = RegistryClient::new(base_url);

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

    let request = handle.join().expect("mock server thread should finish");
    assert!(
        request.starts_with("GET /v2/alpine/tags/list?n=1 HTTP/1.1"),
        "got {request}"
    );
}

#[tokio::test]
async fn registry_client_treats_null_tags_as_empty_list() {
    let (base_url, handle) =
        spawn_registry_mock(MockResponse::json_ok(r#"{"name":"alpine","tags":null}"#));
    let client = RegistryClient::new(base_url);

    let tags = client
        .list_tags(FIXTURE_REPOSITORY, None, None)
        .await
        .expect("null tags response should parse as an empty tag list");

    assert_eq!(tags.name, FIXTURE_REPOSITORY);
    assert!(tags.tags.is_empty());

    let request = handle.join().expect("mock server thread should finish");
    assert!(
        request.starts_with("GET /v2/alpine/tags/list HTTP/1.1"),
        "got {request}"
    );
}

#[tokio::test]
async fn registry_client_resolves_manifest_digest_from_header() {
    let expected_digest = "sha256:0123456789abcdef";
    let (base_url, handle) = spawn_registry_mock(
        MockResponse::empty_ok().with_header("Docker-Content-Digest", expected_digest),
    );
    let client = RegistryClient::new(base_url);

    let digest = client
        .resolve_digest(FIXTURE_REPOSITORY, FIXTURE_TAG)
        .await
        .expect("digest resolution should succeed");

    assert!(
        digest.starts_with("sha256:"),
        "expected sha256 digest, got {digest}"
    );
    assert_eq!(digest, expected_digest);

    let request = handle.join().expect("mock server thread should finish");
    assert!(
        request.starts_with("HEAD /v2/alpine/manifests/latest HTTP/1.1"),
        "got {request}"
    );
}

#[tokio::test]
async fn registry_client_fetches_supported_manifest_media_type() {
    let body = r#"{
        "schemaVersion": 2,
        "mediaType": "application/vnd.docker.distribution.manifest.v2+json",
        "layers": [
            {
                "mediaType": "application/vnd.docker.image.rootfs.diff.tar.gzip",
                "size": 42,
                "digest": "sha256:layer"
            }
        ]
    }"#;
    let (base_url, handle) = spawn_registry_mock(
        MockResponse::new(body).with_header("Content-Type", DOCKER_SCHEMA2_MANIFEST),
    );
    let client = RegistryClient::new(base_url);

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

    let request = handle.join().expect("mock server thread should finish");
    assert!(
        request.starts_with("GET /v2/alpine/manifests/latest HTTP/1.1"),
        "got {request}"
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

struct MockResponse {
    headers: Vec<(&'static str, &'static str)>,
    body: &'static str,
}

impl MockResponse {
    fn new(body: &'static str) -> Self {
        Self {
            headers: Vec::new(),
            body,
        }
    }

    fn empty_ok() -> Self {
        Self::new("")
    }

    fn json_ok(body: &'static str) -> Self {
        Self::new(body).with_header("Content-Type", "application/json")
    }

    fn with_header(mut self, name: &'static str, value: &'static str) -> Self {
        self.headers.push((name, value));
        self
    }
}

fn spawn_registry_mock(response: MockResponse) -> (String, thread::JoinHandle<String>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("mock server should bind");
    let base_url = format!("http://{}", listener.local_addr().expect("mock address"));

    let handle = thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("mock request should connect");
        let mut request_buffer = [0_u8; 2048];
        let bytes_read = stream
            .read(&mut request_buffer)
            .expect("mock request should read");
        let request = String::from_utf8_lossy(&request_buffer[..bytes_read]).to_string();

        let mut raw_response = format!(
            "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n",
            response.body.len()
        );
        for (name, value) in response.headers {
            raw_response.push_str(name);
            raw_response.push_str(": ");
            raw_response.push_str(value);
            raw_response.push_str("\r\n");
        }
        raw_response.push_str("\r\n");
        raw_response.push_str(response.body);

        stream
            .write_all(raw_response.as_bytes())
            .expect("mock response should write");

        request
    });

    (base_url, handle)
}
