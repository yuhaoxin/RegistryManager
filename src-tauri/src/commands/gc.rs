use std::time::Instant;

use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tauri::State;
use tokio::process::Command;
use uuid::Uuid;

use crate::audit::{log_audit_event, AuditAction, AuditEvent};
use crate::docker::DockerClient;
use crate::store::{get_registry_profile, update_pending_gc_records, RegistryProfile};

use super::{AppError, AppState};

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
    let lock = {
        let mut locks = state
            .gc_locks
            .lock()
            .expect("gc lock map mutex should not be poisoned");
        locks
            .entry(profile.container_id.clone())
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

    let snapshot = match inspect_snapshot(&profile.container_id).await {
        Ok(snapshot) => {
            steps.push(step(
                "snapshot",
                "done",
                "Captured original state, image, mounts, env, restart policy and health.",
            ));
            logs.push(format!(
                "[snapshot] image={} state={} mounts={}",
                snapshot.original_image, snapshot.original_state, snapshot.mount_summary
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

    let config_path = config_path(&profile, &snapshot);
    let temp_name = format!("registry-manager-gc-{}", transaction_id.simple());

    let result = async {
        preflight_config(&snapshot, &config_path, &mut logs).await?;
        preflight_help(&snapshot.original_image, &mut logs).await?;
        steps.push(step("preflight", "done", "Verified local Docker context, config readability and garbage-collect help before downtime."));

        if snapshot.original_running {
            docker_output(["stop", &profile.container_id]).await?;
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
            docker_output(["start", &profile.container_id]).await?;
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
            if docker_output(["start", &profile.container_id])
                .await
                .is_ok()
            {
                recovery_action = "restarted_original_container_after_failure".to_string();
            } else {
                recovery_action = format!(
                    "manual recovery required: docker start {}",
                    profile.container_id
                );
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
            container_id: Some(profile.container_id.clone()),
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
    let output = docker_output(["inspect", container_id]).await?;
    let mut values: Vec<Value> =
        serde_json::from_str(&output).map_err(|error| error.to_string())?;
    let value = values
        .pop()
        .ok_or_else(|| "docker inspect returned no container".to_string())?;
    let state = value.get("State").cloned().unwrap_or(Value::Null);
    let config = value.get("Config").cloned().unwrap_or(Value::Null);
    let host_config = value.get("HostConfig").cloned().unwrap_or(Value::Null);
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
    let output = Command::new("docker")
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
    let output = Command::new("docker")
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

fn step(id: &str, status: &str, message: &str) -> GcStep {
    GcStep {
        id: id.to_string(),
        status: status.to_string(),
        message: message.to_string(),
    }
}
