use bollard::Docker;

use super::DockerError;

#[derive(Debug)]
pub struct DockerClient {
    docker: Docker,
}

impl DockerClient {
    pub async fn connect_local() -> Result<Self, DockerError> {
        verify_local_docker_context()?;

        let docker = Docker::connect_with_local_defaults()
            .map_err(|error| DockerError::DockerUnavailable(error.to_string()))?;

        docker
            .version()
            .await
            .map_err(|error| DockerError::DockerUnavailable(error.to_string()))?;

        Ok(Self { docker })
    }

    pub fn docker(&self) -> &Docker {
        &self.docker
    }
}

pub fn verify_local_docker_context() -> Result<(), DockerError> {
    if let Ok(host) = std::env::var("DOCKER_HOST") {
        if is_remote_docker_host(&host) {
            return Err(DockerError::RemoteContext(host));
        }
    }

    if let Ok(context) = std::env::var("DOCKER_CONTEXT") {
        let normalized = context.trim();
        if !normalized.is_empty()
            && !matches!(
                normalized,
                "default" | "desktop-linux" | "colima" | "rancher-desktop"
            )
        {
            return Err(DockerError::RemoteContext(normalized.to_string()));
        }
    }

    Ok(())
}

fn is_remote_docker_host(host: &str) -> bool {
    let value = host.trim().to_ascii_lowercase();
    value.starts_with("tcp://")
        || value.starts_with("http://")
        || value.starts_with("https://")
        || value.starts_with("ssh://")
}

#[cfg(test)]
mod tests {
    use super::is_remote_docker_host;

    #[test]
    fn remote_host_detection_flags_network_hosts() {
        assert!(is_remote_docker_host("tcp://192.168.1.10:2375"));
        assert!(is_remote_docker_host("ssh://docker@example.com"));
        assert!(!is_remote_docker_host("unix:///var/run/docker.sock"));
    }
}
