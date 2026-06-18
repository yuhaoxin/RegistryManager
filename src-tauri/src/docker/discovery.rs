use std::collections::HashMap;

use bollard::container::{InspectContainerOptions, ListContainersOptions};
use bollard::models::{ContainerInspectResponse, PortBinding};

use super::{ContainerMount, ContainerPort, DockerClient, DockerError, RegistryContainerSummary};

pub async fn discover_registry_containers(
    client: &DockerClient,
) -> Result<Vec<RegistryContainerSummary>, DockerError> {
    let options = ListContainersOptions::<String> {
        all: true,
        ..Default::default()
    };
    let containers = client
        .docker()
        .list_containers(Some(options))
        .await
        .map_err(|error| DockerError::DockerUnavailable(error.to_string()))?;

    let mut registries = Vec::new();
    for container in containers {
        let id = container.id.unwrap_or_default();
        if id.is_empty() {
            continue;
        }

        let image = container.image.unwrap_or_default();
        let names = container.names.unwrap_or_default();
        let labels = container.labels.unwrap_or_default();

        if !looks_like_registry(&image, &names, &labels) {
            continue;
        }

        let inspect = client
            .docker()
            .inspect_container(&id, None::<InspectContainerOptions>)
            .await
            .map_err(|source| DockerError::InspectFailed {
                container_id: id.clone(),
                source,
            })?;

        registries.push(summarize_container(inspect)?);
    }

    Ok(registries)
}

pub fn summarize_container(
    inspect: ContainerInspectResponse,
) -> Result<RegistryContainerSummary, DockerError> {
    let id = inspect
        .id
        .clone()
        .ok_or_else(|| DockerError::NotFound("missing id".into()))?;
    let name = inspect
        .name
        .clone()
        .unwrap_or_default()
        .trim_start_matches('/')
        .to_string();
    let image = inspect
        .config
        .as_ref()
        .and_then(|config| config.image.clone())
        .or(inspect.image.clone())
        .unwrap_or_default();
    let env = inspect
        .config
        .as_ref()
        .and_then(|config| config.env.clone())
        .unwrap_or_default();
    let state = inspect
        .state
        .as_ref()
        .and_then(|state| state.status.as_ref().map(|status| status.to_string()));
    let health_status = inspect
        .state
        .as_ref()
        .and_then(|state| state.health.as_ref())
        .and_then(|health| health.status.as_ref().map(|status| status.to_string()));
    let restart_policy = inspect
        .host_config
        .as_ref()
        .and_then(|host_config| host_config.restart_policy.as_ref())
        .and_then(|policy| policy.name.as_ref().map(|name| name.to_string()));

    let ports = inspect
        .network_settings
        .as_ref()
        .and_then(|settings| settings.ports.clone())
        .map(port_map_to_summaries)
        .unwrap_or_default();
    let registry_url = ports
        .iter()
        .find(|port| port.container_port == 5000 && port.host_port.is_some())
        .and_then(|port| {
            port.host_port
                .map(|host_port| format!("http://localhost:{host_port}"))
        });

    let mounts = inspect
        .mounts
        .unwrap_or_default()
        .into_iter()
        .map(|mount| ContainerMount {
            source: mount.source,
            destination: mount.destination,
            mode: mount.mode,
            mount_type: mount.typ.map(|mount_type| mount_type.to_string()),
        })
        .collect();

    Ok(RegistryContainerSummary {
        id,
        name,
        image,
        registry_url,
        ports,
        mounts,
        state,
        env,
        restart_policy,
        health_status,
    })
}

fn looks_like_registry(image: &str, names: &[String], labels: &HashMap<String, String>) -> bool {
    let image = image.to_ascii_lowercase();
    image.contains("registry")
        || names
            .iter()
            .any(|name| name.to_ascii_lowercase().contains("registry"))
        || labels.iter().any(|(key, value)| {
            key.to_ascii_lowercase().contains("registry")
                || value.to_ascii_lowercase().contains("registry")
        })
}

fn port_map_to_summaries(ports: HashMap<String, Option<Vec<PortBinding>>>) -> Vec<ContainerPort> {
    let mut summaries = Vec::new();
    for (container_port, bindings) in ports {
        let (port, protocol) = parse_container_port(&container_port);
        match bindings {
            Some(bindings) if !bindings.is_empty() => {
                for binding in bindings {
                    summaries.push(ContainerPort {
                        container_port: port,
                        host_ip: binding.host_ip,
                        host_port: binding.host_port.and_then(|value| value.parse().ok()),
                        protocol: protocol.clone(),
                    });
                }
            }
            _ => summaries.push(ContainerPort {
                container_port: port,
                host_ip: None,
                host_port: None,
                protocol,
            }),
        }
    }
    summaries.sort_by_key(|port| (port.container_port, port.host_port.unwrap_or_default()));
    summaries
}

fn parse_container_port(value: &str) -> (u16, String) {
    let mut parts = value.split('/');
    let port = parts
        .next()
        .and_then(|part| part.parse().ok())
        .unwrap_or_default();
    let protocol = parts.next().unwrap_or("tcp").to_string();
    (port, protocol)
}
