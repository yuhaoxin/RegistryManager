use reqwest::header::{ACCEPT, CONTENT_TYPE};
use url::Url;

use super::error::RegistryError;
use super::types::{
    Catalog, ImageManifestDocument, LayerSummary, Manifest, ManifestListDocument, PlatformSummary,
    TagDigest, TagsList, DOCKER_MANIFEST_LIST, DOCKER_SCHEMA2_MANIFEST, OCI_IMAGE_INDEX,
    OCI_IMAGE_MANIFEST,
};

const MANIFEST_ACCEPT_HEADER: &str = concat!(
    "application/vnd.docker.distribution.manifest.v2+json",
    ", ",
    "application/vnd.docker.distribution.manifest.list.v2+json",
    ", ",
    "application/vnd.oci.image.manifest.v1+json",
    ", ",
    "application/vnd.oci.image.index.v1+json"
);

#[derive(Debug, Clone)]
pub struct RegistryClient {
    base_url: String,
    http: reqwest::Client,
    auth: Option<RegistryAuth>,
}

#[derive(Debug, Clone)]
struct RegistryAuth {
    username: String,
    password: String,
}

impl RegistryClient {
    pub fn new(base_url: String) -> Self {
        Self {
            base_url,
            http: reqwest::Client::builder()
                .no_proxy()
                .build()
                .unwrap_or_else(|_| reqwest::Client::new()),
            auth: None,
        }
    }

    pub fn with_basic_auth(
        mut self,
        username: impl Into<String>,
        password: impl Into<String>,
    ) -> Self {
        self.auth = Some(RegistryAuth {
            username: username.into(),
            password: password.into(),
        });
        self
    }

    pub async fn ping(&self) -> Result<(), RegistryError> {
        let response = self
            .authorize(self.http.get(self.url(&["v2", ""], &[])?))
            .send()
            .await?;
        Self::ensure_success(response.status())?;
        Ok(())
    }

    pub async fn list_catalog(
        &self,
        n: Option<u32>,
        last: Option<String>,
    ) -> Result<Catalog, RegistryError> {
        let response = self
            .authorize(
                self.http
                    .get(self.url(&["v2", "_catalog"], &pagination_query(n, last))?),
            )
            .send()
            .await?;
        Self::ensure_success(response.status())?;
        Ok(serde_json::from_slice(&response.bytes().await?)?)
    }

    pub async fn list_tags(
        &self,
        name: &str,
        n: Option<u32>,
        last: Option<String>,
    ) -> Result<TagsList, RegistryError> {
        let mut segments = vec!["v2"];
        segments.extend(name.split('/').filter(|segment| !segment.is_empty()));
        segments.extend(["tags", "list"]);

        let response = self
            .authorize(
                self.http
                    .get(self.url(&segments, &pagination_query(n, last))?),
            )
            .send()
            .await?;
        Self::ensure_success(response.status())?;
        Ok(serde_json::from_slice(&response.bytes().await?)?)
    }

    pub async fn fetch_manifest(
        &self,
        name: &str,
        reference: &str,
    ) -> Result<Manifest, RegistryError> {
        let (content_type, bytes) = self.fetch_manifest_raw(name, reference).await?;
        manifest_from_bytes(content_type, bytes)
    }

    pub async fn fetch_manifest_raw(
        &self,
        name: &str,
        reference: &str,
    ) -> Result<(String, Vec<u8>), RegistryError> {
        let response = self
            .authorize(self.http.get(self.manifest_url(name, reference)?))
            .header(ACCEPT, MANIFEST_ACCEPT_HEADER)
            .send()
            .await?;
        Self::ensure_success(response.status())?;

        let content_type = response_content_type(&response)?;
        if !is_supported_manifest_media_type(&content_type) {
            return Err(RegistryError::UnsupportedMediaType(content_type));
        }

        let bytes = response.bytes().await?.to_vec();
        Ok((content_type, bytes))
    }

    pub async fn resolve_digest(
        &self,
        name: &str,
        reference: &str,
    ) -> Result<String, RegistryError> {
        let response = self
            .authorize(self.http.head(self.manifest_url(name, reference)?))
            .header(ACCEPT, MANIFEST_ACCEPT_HEADER)
            .send()
            .await?;
        Self::ensure_success(response.status())?;

        response
            .headers()
            .get("Docker-Content-Digest")
            .and_then(|value| value.to_str().ok())
            .map(str::to_owned)
            .ok_or(RegistryError::DigestNotFound)
    }

    pub async fn resolve_tag_digest(
        &self,
        name: &str,
        tag: &str,
    ) -> Result<TagDigest, RegistryError> {
        Ok(TagDigest {
            tag: tag.to_string(),
            digest: self.resolve_digest(name, tag).await?,
        })
    }

    pub async fn delete_manifest(&self, name: &str, digest: &str) -> Result<(), RegistryError> {
        let response = self
            .authorize(self.http.delete(self.manifest_url(name, digest)?))
            .header(ACCEPT, MANIFEST_ACCEPT_HEADER)
            .send()
            .await?;
        Self::ensure_success(response.status())
    }

    fn authorize(&self, request: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        match &self.auth {
            Some(auth) => request.basic_auth(&auth.username, Some(&auth.password)),
            None => request,
        }
    }

    fn manifest_url(&self, name: &str, reference: &str) -> Result<Url, RegistryError> {
        let mut segments = vec!["v2"];
        segments.extend(name.split('/').filter(|segment| !segment.is_empty()));
        segments.extend(["manifests", reference]);
        self.url(&segments, &[])
    }

    fn url(&self, segments: &[&str], query: &[(String, String)]) -> Result<Url, RegistryError> {
        let mut url = Url::parse(&self.base_url).map_err(|_| RegistryError::InvalidUrl)?;
        {
            let mut path = url
                .path_segments_mut()
                .map_err(|_| RegistryError::InvalidUrl)?;
            path.clear();
            for segment in segments {
                path.push(segment);
            }
        }

        if !query.is_empty() {
            let mut pairs = url.query_pairs_mut();
            for (key, value) in query {
                pairs.append_pair(key, value);
            }
        }

        Ok(url)
    }

    fn ensure_success(status: reqwest::StatusCode) -> Result<(), RegistryError> {
        if status.is_success() {
            return Ok(());
        }

        if status == reqwest::StatusCode::NOT_FOUND {
            return Err(RegistryError::NotFound);
        }
        if status == reqwest::StatusCode::UNAUTHORIZED {
            return Err(RegistryError::Unauthorized);
        }
        if status == reqwest::StatusCode::FORBIDDEN {
            return Err(RegistryError::Forbidden);
        }

        Err(RegistryError::UnexpectedStatus(status.as_u16()))
    }
}

fn pagination_query(n: Option<u32>, last: Option<String>) -> Vec<(String, String)> {
    let mut query = Vec::new();
    if let Some(n) = n {
        query.push(("n".to_string(), n.to_string()));
    }
    if let Some(last) = last {
        query.push(("last".to_string(), last));
    }
    query
}

fn response_content_type(response: &reqwest::Response) -> Result<String, RegistryError> {
    response
        .headers()
        .get(CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .map(|value| value.split(';').next().unwrap_or(value).trim().to_string())
        .filter(|value| !value.is_empty())
        .ok_or_else(|| RegistryError::UnsupportedMediaType(String::new()))
}

fn is_supported_manifest_media_type(content_type: &str) -> bool {
    matches!(
        content_type,
        DOCKER_SCHEMA2_MANIFEST | DOCKER_MANIFEST_LIST | OCI_IMAGE_MANIFEST | OCI_IMAGE_INDEX
    )
}

fn manifest_from_bytes(content_type: String, bytes: Vec<u8>) -> Result<Manifest, RegistryError> {
    match content_type.as_str() {
        DOCKER_SCHEMA2_MANIFEST => {
            let document: ImageManifestDocument = serde_json::from_slice(&bytes)?;
            Ok(Manifest::DockerSchema2V2 {
                content_type,
                layers: document
                    .layers
                    .into_iter()
                    .map(LayerSummary::from)
                    .collect(),
            })
        }
        DOCKER_MANIFEST_LIST => {
            let document: ManifestListDocument = serde_json::from_slice(&bytes)?;
            Ok(Manifest::DockerManifestList {
                content_type,
                platforms: platforms_from_manifest_list(document),
            })
        }
        OCI_IMAGE_MANIFEST => {
            let document: ImageManifestDocument = serde_json::from_slice(&bytes)?;
            Ok(Manifest::OciImageManifest {
                content_type,
                layers: document
                    .layers
                    .into_iter()
                    .map(LayerSummary::from)
                    .collect(),
            })
        }
        OCI_IMAGE_INDEX => {
            let document: ManifestListDocument = serde_json::from_slice(&bytes)?;
            Ok(Manifest::OciImageIndex {
                content_type,
                platforms: platforms_from_manifest_list(document),
            })
        }
        _ => Ok(Manifest::Raw {
            content_type,
            bytes,
        }),
    }
}

fn platforms_from_manifest_list(document: ManifestListDocument) -> Vec<PlatformSummary> {
    document
        .manifests
        .into_iter()
        .filter_map(|descriptor| descriptor.platform.map(PlatformSummary::from))
        .collect()
}
