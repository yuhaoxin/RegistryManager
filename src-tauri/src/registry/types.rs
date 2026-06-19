use serde::{Deserialize, Deserializer};

pub const DOCKER_SCHEMA2_MANIFEST: &str = "application/vnd.docker.distribution.manifest.v2+json";
pub const DOCKER_MANIFEST_LIST: &str = "application/vnd.docker.distribution.manifest.list.v2+json";
pub const OCI_IMAGE_MANIFEST: &str = "application/vnd.oci.image.manifest.v1+json";
pub const OCI_IMAGE_INDEX: &str = "application/vnd.oci.image.index.v1+json";

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct Catalog {
    pub repositories: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct TagsList {
    pub name: String,
    #[serde(default, deserialize_with = "null_as_empty_tags")]
    pub tags: Vec<String>,
}

fn null_as_empty_tags<'de, D>(deserializer: D) -> Result<Vec<String>, D::Error>
where
    D: Deserializer<'de>,
{
    Ok(Option::<Vec<String>>::deserialize(deserializer)?.unwrap_or_default())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TagDigest {
    pub tag: String,
    pub digest: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Manifest {
    DockerSchema2V1 {
        content_type: String,
        layers: Vec<LayerSummary>,
    },
    DockerSchema2V2 {
        content_type: String,
        layers: Vec<LayerSummary>,
    },
    DockerManifestList {
        content_type: String,
        platforms: Vec<PlatformSummary>,
    },
    OciImageManifest {
        content_type: String,
        layers: Vec<LayerSummary>,
    },
    OciImageIndex {
        content_type: String,
        platforms: Vec<PlatformSummary>,
    },
    Raw {
        content_type: String,
        bytes: Vec<u8>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LayerSummary {
    pub digest: String,
    pub size: i64,
    pub media_type: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlatformSummary {
    pub os: Option<String>,
    pub architecture: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ImageManifestDocument {
    #[allow(dead_code)]
    pub schema_version: i64,
    #[allow(dead_code)]
    pub media_type: Option<String>,
    #[serde(default)]
    pub layers: Vec<Descriptor>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ManifestListDocument {
    #[allow(dead_code)]
    pub schema_version: i64,
    #[allow(dead_code)]
    pub media_type: Option<String>,
    #[serde(default)]
    pub manifests: Vec<Descriptor>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Descriptor {
    pub media_type: Option<String>,
    pub size: Option<i64>,
    pub digest: Option<String>,
    pub platform: Option<PlatformDocument>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct PlatformDocument {
    pub os: Option<String>,
    pub architecture: Option<String>,
}

impl From<Descriptor> for LayerSummary {
    fn from(value: Descriptor) -> Self {
        Self {
            digest: value.digest.unwrap_or_default(),
            size: value.size.unwrap_or_default(),
            media_type: value.media_type.unwrap_or_default(),
        }
    }
}

impl From<PlatformDocument> for PlatformSummary {
    fn from(value: PlatformDocument) -> Self {
        Self {
            os: value.os,
            architecture: value.architecture,
        }
    }
}

impl Manifest {
    pub fn content_type(&self) -> &str {
        match self {
            Self::DockerSchema2V1 { content_type, .. }
            | Self::DockerSchema2V2 { content_type, .. }
            | Self::DockerManifestList { content_type, .. }
            | Self::OciImageManifest { content_type, .. }
            | Self::OciImageIndex { content_type, .. }
            | Self::Raw { content_type, .. } => content_type,
        }
    }
}
