mod client;
mod error;

pub use client::{verify_local_docker_context, DockerClient};
pub use error::DockerError;
