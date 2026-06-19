use std::path::PathBuf;
use std::time::Instant;

use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tauri::State;
use tokio::process::Command;
use url::Url;
use uuid::Uuid;

use crate::audit::{log_audit_event, AuditAction, AuditEvent};
use crate::docker::DockerClient;
use crate::store::{get_registry_profile, update_pending_gc_records, RegistryProfile};

use super::{registry::ensure_local_registry_target, AppError, AppState};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct GcStep {
    pub id: String,
    pub status: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct GcResult {
    pub transaction_id: String,
    pub status: String,
    pub exit_code: Option<i32>,
    pub duration_ms: i64,
    pub logs: Vec<String>,
    pub steps: Vec<GcStep>,
    pub original_state: String,
    pub original_image: String,
    pub mount_summary: String,
    pub config_path: String,
    pub recovery_action: String,
    pub final_health_status: String,
}

#[derive(Debug, Clone)]
struct Snapshot {
    container_id: String,
    container_name: Option<String>,
    original_state: String,
    original_running: bool,
    original_image: String,
    env: Vec<String>,
    mounts: Vec<Value>,
    mount_summary: String,
    restart_policy: Option<Value>,
    health: Option<Value>,
}

#[tauri::command]
pub async fn run_local_gc(
    profile_id: String,
    confirm_downtime: bool,
    state: State<'_, AppState>,
) -> Result<GcResult, AppError> {
    if !confirm_downtime {
        return Err(AppError::new(
            "gc_confirmation_required",
            "Local GC requires explicit downtime confirmation.",
        ));
    }

    let profile = require_profile(&state, &profile_id).await?;
    ensure_local_registry_target(&profile.registry_url).await?;
    let lock_key = format!("registry-url:{}", profile.registry_url);
    let lock = {
        let mut locks = state
            .gc_locks
            .lock()
            .expect("gc lock map mutex should not be poisoned");
        locks
            .entry(lock_key)
            .or_insert_with(|| std::sync::Arc::new(tokio::sync::Mutex::new(())))
            .clone()
    };
    let _guard = lock.lock().await;

    DockerClient::connect_local().await?;

    let transaction_id = Uuid::new_v4();
    let started = Instant::now();
    let mut logs = Vec::new();
    let mut steps = Vec::new();
    let mut exit_code = None;
    let mut recovery_action = "none".to_string();
    let mut final_health_status = "not_checked".to_string();

    let snapshot = match inspect_profile_container(&profile).await {
        Ok(snapshot) => {
            steps.push(step(
                "snapshot",
                "done",
                "Captured original state, image, mounts, env, restart policy and health.",
            ));
            logs.push(format!(
                "[snapshot] container={} image={} state={} mounts={}",
                snapshot
                    .container_name
                    .as_deref()
                    .unwrap_or(&snapshot.container_id),
                snapshot.original_image,
                snapshot.original_state,
                snapshot.mount_summary
            ));
            snapshot
        }
        Err(error) => {
            return Err(AppError::with_details(
                "gc_snapshot_failed",
                "Failed to inspect the registry container before GC.",
                error,
            ))
        }
    };
    let container_id = snapshot.container_id.clone();

    let config_path = config_path(&profile, &snapshot);
    let temp_name = format!("registry-manager-gc-{}", transaction_id.simple());

    let result = async {
        preflight_config(&snapshot, &config_path, &mut logs).await?;
        preflight_help(&snapshot.original_image, &mut logs).await?;
        steps.push(step("preflight", "done", "Verified local Docker context, config readability and garbage-collect help before downtime."));

        if snapshot.original_running {
            docker_output(["stop", &container_id]).await?;
            logs.push("[stop] original registry container stopped".to_string());
            steps.push(step("stop", "done", "Stopped original registry container before offline GC."));
        } else {
            steps.push(step("stop", "skipped", "Original registry container was not running; stopped state will be preserved."));
        }

        let (code, output) = run_gc_container(&snapshot, &temp_name).await?;
        exit_code = Some(code);
        logs.extend(output.lines().map(str::to_string));
        steps.push(step(
            "gc",
            if code == 0 { "done" } else { "failed" },
            "Ran registry garbage-collect --delete-untagged /etc/docker/registry/config.yml in a temporary container.",
        ));

        cleanup_temp(&temp_name, &mut logs).await;
        steps.push(step("cleanup", "done", "Temporary GC container removed."));

        if snapshot.original_running {
            docker_output(["start", &container_id]).await?;
            recovery_action = "restarted_original_container".to_string();
            steps.push(step("restart", "done", "Original registry container restarted."));
            health_check(&profile.registry_url).await?;
            final_health_status = "healthy".to_string();
            steps.push(step("health", "done", "/v2/ health check passed after restart."));
        } else {
            recovery_action = "left_original_container_stopped".to_string();
            final_health_status = "not_applicable_originally_stopped".to_string();
            steps.push(step("restart", "skipped", "Original registry container was stopped before GC and remains stopped."));
            steps.push(step("health", "skipped", "Health check skipped because original container was stopped."));
        }

        if code == 0 {
            Ok(())
        } else {
            Err(format!("registry garbage-collect exited with code {code}"))
        }
    }
    .await;

    if result.is_err() {
        cleanup_temp(&temp_name, &mut logs).await;
        if snapshot.original_running {
            if docker_output(["start", &container_id]).await.is_ok() {
                recovery_action = "restarted_original_container_after_failure".to_string();
            } else {
                recovery_action =
                    format!("manual recovery required: docker start {}", container_id);
            }
        }
        final_health_status = "gc_failed".to_string();
        steps.push(step("failure_recovery", "done", &recovery_action));
    }

    let status = if result.is_ok() {
        "gc_completed"
    } else {
        "gc_failed"
    };
    update_pending_gc_records(&state.pool, profile.id, status).await?;

    let duration_ms = started.elapsed().as_millis() as i64;
    let log_excerpt = serde_json::json!({
        "originalState": snapshot.original_state,
        "originalImage": snapshot.original_image,
        "mountSummary": snapshot.mount_summary,
        "configPath": config_path,
        "restartPolicy": snapshot.restart_policy,
        "health": snapshot.health,
        "recoveryAction": recovery_action,
        "finalHealthStatus": final_health_status,
        "exitCode": exit_code,
        "logs": logs.iter().rev().take(25).cloned().collect::<Vec<_>>(),
    })
    .to_string();

    log_audit_event(
        &state.pool,
        &AuditEvent {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            action: AuditAction::LocalGc,
            registry_id: Some(profile.id),
            container_id: Some(container_id),
            repository_name: None,
            tag: None,
            digest: None,
            status: status.to_string(),
            duration_ms: Some(duration_ms),
            error_message: result.as_ref().err().cloned(),
            log_excerpt: Some(log_excerpt),
        },
    )
    .await?;

    if let Err(error) = result {
        return Err(AppError::with_details(
            "local_gc_failed",
            "Local registry GC failed and recovery was attempted.",
            error,
        ));
    }

    Ok(GcResult {
        transaction_id: transaction_id.to_string(),
        status: status.to_string(),
        exit_code,
        duration_ms,
        logs,
        steps,
        original_state: snapshot.original_state,
        original_image: snapshot.original_image,
        mount_summary: snapshot.mount_summary,
        config_path,
        recovery_action,
        final_health_status,
    })
}

async fn require_profile(state: &AppState, profile_id: &str) -> Result<RegistryProfile, AppError> {
    let id = Uuid::parse_str(profile_id)?;
    get_registry_profile(&state.pool, id).await?.ok_or_else(|| {
        AppError::new(
            "profile_not_found",
            "Selected registry profile was not found.",
        )
    })
}

async fn inspect_snapshot(container_id: &str) -> Result<Snapshot, String> {
    let value = inspect_container_value(container_id).await?;
    let state = value.get("State").cloned().unwrap_or(Value::Null);
    let config = value.get("Config").cloned().unwrap_or(Value::Null);
    let host_config = value.get("HostConfig").cloned().unwrap_or(Value::Null);
    let resolved_container_id = value
        .get("Id")
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .unwrap_or(container_id)
        .to_string();
    let resolved_container_name = value
        .get("Name")
        .and_then(Value::as_str)
        .map(|value| value.trim_start_matches('/').to_string())
        .filter(|value| !value.is_empty());
    let mounts = value
        .get("Mounts")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let original_state = state
        .get("Status")
        .and_then(Value::as_str)
        .unwrap_or("unknown")
        .to_string();
    let original_running = state
        .get("Running")
        .and_then(Value::as_bool)
        .unwrap_or(original_state == "running");
    let original_image = config
        .get("Image")
        .and_then(Value::as_str)
        .or_else(|| value.get("Image").and_then(Value::as_str))
        .unwrap_or("registry:2")
        .to_string();
    let env = config
        .get("Env")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default();
    let mount_summary = serde_json::to_string(&mounts).map_err(|error| error.to_string())?;

    Ok(Snapshot {
        container_id: resolved_container_id,
        container_name: resolved_container_name,
        original_state,
        original_running,
        original_image,
        env,
        mounts,
        mount_summary,
        restart_policy: host_config.get("RestartPolicy").cloned(),
        health: state.get("Health").cloned(),
    })
}

async fn inspect_container_value(container_ref: &str) -> Result<Value, String> {
    let output = docker_output(["inspect", container_ref]).await?;
    let mut values: Vec<Value> =
        serde_json::from_str(&output).map_err(|error| error.to_string())?;
    values
        .pop()
        .ok_or_else(|| "docker inspect returned no container".to_string())
}

async fn inspect_profile_container(profile: &RegistryProfile) -> Result<Snapshot, String> {
    let mut errors = Vec::new();
    let mut candidates = Vec::new();

    match discover_registry_container_ref(&profile.registry_url).await {
        Ok(container_ref) => candidates.push(("registry URL port mapping", container_ref)),
        Err(error) => errors.push(format!("registry URL discovery failed: {}", error.trim())),
    }

    push_optional_container_ref(
        &mut candidates,
        "container name",
        profile.container_name.as_deref(),
    );
    push_optional_container_ref(
        &mut candidates,
        "legacy container id",
        profile.container_id.as_deref(),
    );

    for (source, container_ref) in candidates {
        match inspect_snapshot(&container_ref).await {
            Ok(snapshot) => return Ok(snapshot),
            Err(error) => errors.push(format!("{source} inspect failed: {}", error.trim())),
        }
    }

    if errors.is_empty() {
        Err("No Docker container publishes the selected registry URL port.".to_string())
    } else {
        Err(errors.join("; "))
    }
}

async fn discover_registry_container_ref(registry_url: &str) -> Result<String, String> {
    let (host, port) = registry_endpoint(registry_url)?;
    let output = docker_output_owned(vec![
        "ps".to_string(),
        "-a".to_string(),
        "--filter".to_string(),
        format!("publish={port}"),
        "--format".to_string(),
        "{{.ID}}".to_string(),
    ])
    .await?;
    let candidate_ids = output
        .lines()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .collect::<Vec<_>>();

    if candidate_ids.is_empty() {
        return Err(format!("No Docker container publishes host port {port}."));
    }

    let mut matches = Vec::new();
    for candidate_id in candidate_ids {
        let value = inspect_container_value(candidate_id).await?;
        if container_publishes_registry_port(&value, &host, port) {
            matches.push(value);
        }
    }

    let running = matches
        .iter()
        .filter(|value| {
            value
                .get("State")
                .and_then(|state| state.get("Running"))
                .and_then(Value::as_bool)
                .unwrap_or(false)
        })
        .collect::<Vec<_>>();

    let selected = if running.len() == 1 {
        running[0]
    } else if running.is_empty() && matches.len() == 1 {
        &matches[0]
    } else if matches.is_empty() {
        return Err(format!(
            "No Docker container port binding matches {host}:{port}."
        ));
    } else {
        return Err(format!(
            "Multiple Docker containers publish host port {port}; cannot choose a registry container safely."
        ));
    };

    selected
        .get("Id")
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .ok_or_else(|| "docker inspect did not return a container id".to_string())
}

fn registry_endpoint(registry_url: &str) -> Result<(String, u16), String> {
    let url = Url::parse(registry_url).map_err(|error| error.to_string())?;
    let host = url
        .host_str()
        .ok_or_else(|| "registry URL is missing a host".to_string())?
        .to_string();
    let port = url
        .port_or_known_default()
        .ok_or_else(|| "registry URL is missing a port".to_string())?;
    Ok((host, port))
}

fn container_publishes_registry_port(
    value: &Value,
    registry_host: &str,
    registry_port: u16,
) -> bool {
    value
        .get("NetworkSettings")
        .and_then(|settings| settings.get("Ports"))
        .and_then(Value::as_object)
        .map(|ports| {
            ports.values().any(|bindings| {
                bindings.as_array().is_some_and(|items| {
                    items.iter().any(|binding| {
                        let Some(host_port) = binding
                            .get("HostPort")
                            .and_then(Value::as_str)
                            .and_then(|value| value.parse::<u16>().ok())
                        else {
                            return false;
                        };
                        host_port == registry_port
                            && host_binding_matches(
                                registry_host,
                                binding.get("HostIp").and_then(Value::as_str),
                            )
                    })
                })
            })
        })
        .unwrap_or(false)
}

fn host_binding_matches(registry_host: &str, host_ip: Option<&str>) -> bool {
    let binding = host_ip.unwrap_or_default().trim();
    if matches!(binding, "" | "0.0.0.0" | "::") {
        return true;
    }

    let registry_host = registry_host
        .trim()
        .trim_start_matches('[')
        .trim_end_matches(']');
    if registry_host.eq_ignore_ascii_case("localhost") {
        return matches!(binding, "127.0.0.1" | "::1" | "localhost");
    }

    binding.eq_ignore_ascii_case(registry_host)
}

fn push_optional_container_ref(
    candidates: &mut Vec<(&'static str, String)>,
    source: &'static str,
    value: Option<&str>,
) {
    let Some(container_ref) = value.map(str::trim).filter(|value| !value.is_empty()) else {
        return;
    };
    if candidates
        .iter()
        .any(|(_, existing)| same_container_ref(existing, container_ref))
    {
        return;
    }
    candidates.push((source, container_ref.to_string()));
}

fn same_container_ref(left: &str, right: &str) -> bool {
    left.trim().trim_start_matches('/') == right.trim().trim_start_matches('/')
}

fn config_path(profile: &RegistryProfile, snapshot: &Snapshot) -> String {
    snapshot
        .env
        .iter()
        .find_map(|value| {
            value
                .strip_prefix("REGISTRY_CONFIGURATION_PATH=")
                .map(str::to_string)
        })
        .or_else(|| profile.config_path.clone())
        .unwrap_or_else(|| "/etc/docker/registry/config.yml".to_string())
}

async fn preflight_config(
    snapshot: &Snapshot,
    path: &str,
    logs: &mut Vec<String>,
) -> Result<(), String> {
    let mut args = vec!["run".to_string(), "--rm".to_string()];
    args.extend(mount_args(snapshot));
    args.extend([
        "--entrypoint".to_string(),
        "cat".to_string(),
        snapshot.original_image.clone(),
        path.to_string(),
    ]);
    docker_output_owned(args).await.map(|_| {
        logs.push(format!("[preflight] config readable at {path}"));
    })
}

async fn preflight_help(image: &str, logs: &mut Vec<String>) -> Result<(), String> {
    docker_output([
        "run",
        "--rm",
        "--entrypoint",
        "registry",
        image,
        "garbage-collect",
        "--help",
    ])
    .await
    .map(|_| logs.push("[preflight] registry garbage-collect --help succeeded".to_string()))
}

async fn run_gc_container(snapshot: &Snapshot, temp_name: &str) -> Result<(i32, String), String> {
    let mut args = vec![
        "run".to_string(),
        "--name".to_string(),
        temp_name.to_string(),
    ];
    args.extend(mount_args(snapshot));
    args.extend([
        "--entrypoint".to_string(),
        "registry".to_string(),
        snapshot.original_image.clone(),
        "garbage-collect".to_string(),
        "--delete-untagged".to_string(),
        "/etc/docker/registry/config.yml".to_string(),
    ]);
    let output = docker_command()
        .args(args)
        .output()
        .await
        .map_err(|error| error.to_string())?;
    let code = output.status.code().unwrap_or(1);
    let mut text = String::new();
    text.push_str(&String::from_utf8_lossy(&output.stdout));
    text.push_str(&String::from_utf8_lossy(&output.stderr));
    Ok((code, text))
}

async fn cleanup_temp(temp_name: &str, logs: &mut Vec<String>) {
    if docker_output(["rm", "-f", temp_name]).await.is_ok() {
        logs.push(format!("[cleanup] removed temp container {temp_name}"));
    }
}

async fn health_check(registry_url: &str) -> Result<(), String> {
    let url = format!("{}/v2/", registry_url.trim_end_matches('/'));
    reqwest::Client::new()
        .get(url)
        .send()
        .await
        .map_err(|error| error.to_string())?
        .error_for_status()
        .map_err(|error| error.to_string())?;
    Ok(())
}

fn mount_args(snapshot: &Snapshot) -> Vec<String> {
    snapshot
        .mounts
        .iter()
        .filter_map(mount_arg)
        .flat_map(|spec| ["--mount".to_string(), spec])
        .collect()
}

fn mount_arg(mount: &Value) -> Option<String> {
    let typ = mount.get("Type")?.as_str()?;
    let source = if typ == "volume" {
        mount.get("Name").and_then(Value::as_str)
    } else {
        mount.get("Source").and_then(Value::as_str)
    }?;
    let target = mount.get("Destination")?.as_str()?;
    let mut parts = vec![
        format!("type={typ}"),
        format!("source={source}"),
        format!("target={target}"),
    ];
    if mount.get("RW").and_then(Value::as_bool) == Some(false) {
        parts.push("readonly".to_string());
    }
    Some(parts.join(","))
}

async fn docker_output<const N: usize>(args: [&str; N]) -> Result<String, String> {
    let owned = args.into_iter().map(str::to_string).collect::<Vec<_>>();
    docker_output_owned(owned).await
}

async fn docker_output_owned(args: Vec<String>) -> Result<String, String> {
    let output = docker_command()
        .args(args)
        .output()
        .await
        .map_err(|error| error.to_string())?;
    if !output.status.success() {
        let mut text = String::from_utf8_lossy(&output.stderr).to_string();
        if text.trim().is_empty() {
            text = String::from_utf8_lossy(&output.stdout).to_string();
        }
        return Err(text);
    }
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

fn docker_command() -> Command {
    Command::new(docker_program())
}

fn docker_program() -> PathBuf {
    let executable = docker_executable_name();
    if executable_exists_on_path(executable) {
        return PathBuf::from(executable);
    }

    docker_cli_candidates()
        .iter()
        .map(PathBuf::from)
        .find(|path| path.is_file())
        .unwrap_or_else(|| PathBuf::from(executable))
}

fn docker_executable_name() -> &'static str {
    if cfg!(windows) {
        "docker.exe"
    } else {
        "docker"
    }
}

fn executable_exists_on_path(executable: &str) -> bool {
    std::env::var_os("PATH")
        .map(|paths| {
            std::env::split_paths(&paths).any(|directory| directory.join(executable).is_file())
        })
        .unwrap_or(false)
}

fn docker_cli_candidates() -> &'static [&'static str] {
    if cfg!(windows) {
        &[r"C:\Program Files\Docker\Docker\resources\bin\docker.exe"]
    } else {
        &[
            "/usr/local/bin/docker",
            "/opt/homebrew/bin/docker",
            "/usr/bin/docker",
        ]
    }
}

fn step(id: &str, status: &str, message: &str) -> GcStep {
    GcStep {
        id: id.to_string(),
        status: status.to_string(),
        message: message.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{
        container_publishes_registry_port, docker_cli_candidates, docker_executable_name,
        docker_program, inspect_profile_container, registry_endpoint, same_container_ref,
    };
    use crate::store::RegistryProfile;

    #[test]
    fn docker_program_resolves_to_executable_name_or_known_absolute_path() {
        let program = docker_program();
        let program_text = program.to_string_lossy();

        assert!(
            program_text.ends_with(docker_executable_name())
                || docker_cli_candidates()
                    .iter()
                    .any(|candidate| candidate == &program_text.as_ref())
        );
    }

    #[test]
    fn same_container_ref_ignores_leading_slash() {
        assert!(same_container_ref(
            "registry_manager-registry-1",
            "/registry_manager-registry-1"
        ));
        assert!(!same_container_ref(
            "old-container-id",
            "registry_manager-registry-1"
        ));
    }

    #[test]
    fn registry_endpoint_reads_default_and_explicit_ports() {
        assert_eq!(
            registry_endpoint("http://localhost:5001").expect("explicit port should parse"),
            ("localhost".to_string(), 5001)
        );
        assert_eq!(
            registry_endpoint("https://127.0.0.1").expect("known default port should parse"),
            ("127.0.0.1".to_string(), 443)
        );
    }

    #[test]
    fn container_port_match_accepts_loopback_registry_on_all_interfaces() {
        let inspect = json!({
            "NetworkSettings": {
                "Ports": {
                    "5000/tcp": [
                        { "HostIp": "0.0.0.0", "HostPort": "5001" },
                        { "HostIp": "::", "HostPort": "5001" }
                    ]
                }
            }
        });

        assert!(container_publishes_registry_port(
            &inspect,
            "localhost",
            5001
        ));
        assert!(!container_publishes_registry_port(
            &inspect,
            "localhost",
            5002
        ));
    }

    #[tokio::test]
    #[ignore = "requires docker-compose.test.yml registry fixture on localhost:5001"]
    async fn discovers_current_registry_container_by_published_port() {
        let now = chrono::Utc::now();
        let profile = RegistryProfile {
            id: uuid::Uuid::new_v4(),
            name: "Custom profile name, not a container name".to_string(),
            registry_url: "http://127.0.0.1:5001".to_string(),
            credential_ref: None,
            created_at: now,
            updated_at: now,
            container_id: Some("stale-container-id".to_string()),
            container_name: None,
            config_path: None,
        };

        let snapshot = inspect_profile_container(&profile)
            .await
            .expect("registry fixture container should be discovered from the URL port");

        assert_ne!(snapshot.container_id, "stale-container-id");
        assert_eq!(
            snapshot.container_name.as_deref(),
            Some("registry_manager-registry-1")
        );
    }
}
