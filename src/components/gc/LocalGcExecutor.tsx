import { useState } from "react";
import { runTauriCommand } from "../../hooks/useTauriCommand";
import type { GcResult } from "../../types";
import { GcConfirmDialog } from "./GcConfirmDialog";
import { GcLiveLogPanel } from "./GcLiveLogPanel";
import { GcPreflightList } from "./GcPreflightList";
import { GcResultSummary } from "./GcResultSummary";
import { GcStepTimeline } from "./GcStepTimeline";

export interface LocalGcExecutorProps {
  containerName: string;
  profileId?: string;
}

const placeholderPreflight = [
  { name: "Docker daemon", status: "ok" as const, message: "Local Docker daemon is reachable" },
  { name: "Container state", status: "ok" as const, message: "Target container exists" },
  { name: "Storage mounts", status: "ok" as const, message: "Mounts can be reused in temp container" },
  { name: "Registry config", status: "warn" as const, message: "Using default /etc/docker/registry/config.yml" },
];

const placeholderSteps = [
  { id: "snapshot", title: "Snapshot original state", status: "pending" as const, note: "Waiting to capture container config and mounts" },
  { id: "stop", title: "Stop original container", status: "pending" as const, note: "Runs only after confirmation" },
  { id: "gc", title: "Run garbage-collect", status: "pending" as const, note: "Temporary GC container has not started" },
  { id: "cleanup", title: "Remove temp container", status: "pending" as const },
  { id: "restart", title: "Restart registry", status: "pending" as const, note: "Only if originally running" },
  { id: "health", title: "/v2/ health check", status: "pending" as const },
];

const placeholderLogs = [
  "[preflight] Docker context: default",
  "[preflight] Container registry exists (id: abc123)",
  "[gc] registry garbage-collect --delete-untagged /etc/docker/registry/config.yml",
  "[gc] Deleting blob: sha256:abc…",
  "[gc] Deleting blob: sha256:def…",
];

export function LocalGcExecutor({ containerName, profileId }: LocalGcExecutorProps) {
  const [showConfirm, setShowConfirm] = useState(false);
  const [result, setResult] = useState<GcResult | undefined>();
  const [error, setError] = useState<string | undefined>();
  const [running, setRunning] = useState(false);

  async function runGc() {
    setShowConfirm(false);
    setRunning(true);
    setError(undefined);
    try {
      const output = await runTauriCommand<GcResult>("run_local_gc", { profileId, confirmDowntime: true });
      setResult(output);
    } catch (err) {
      setError(errorMessage(err));
    } finally {
      setRunning(false);
    }
  }

  return (
    <div className="card" data-testid="rm-local-gc-executor">
      <div className="card-header">
        <div className="card-title">🧹 Local GC executor</div>
        <button type="button" className="btn btn-danger btn-sm" onClick={() => setShowConfirm(true)} disabled={running}>
          {running ? "Running GC…" : "Run GC"}
        </button>
      </div>
      <div className="card-body">
        <GcPreflightList items={placeholderPreflight} />
        <GcStepTimeline steps={result?.steps.map((item) => ({ id: item.id, title: item.id, status: mapStepStatus(item.status), note: item.message })) ?? placeholderSteps} />
        <GcLiveLogPanel logs={result?.logs ?? placeholderLogs} />
        {error ? <div className="preflight-item error" role="alert">{error}</div> : null}
        {result?.status === "gc_failed" ? (
          <div className="preflight-item error" role="alert">
            Recovery required: {result.recoveryAction}
          </div>
        ) : null}
        <GcResultSummary
          status={running ? "running" : result?.status === "gc_completed" ? "success" : result?.status === "gc_failed" || error ? "failure" : "idle"}
          durationMs={result?.durationMs}
          errorMessage={error ?? (result?.status === "gc_failed" ? result.recoveryAction : undefined)}
        />
      </div>

      <GcConfirmDialog
        open={showConfirm}
        containerName={containerName}
        onConfirm={runGc}
        onCancel={() => setShowConfirm(false)}
      />
    </div>
  );
}

function mapStepStatus(status: string) {
  if (status === "done" || status === "skipped") return "done" as const;
  if (status === "failed") return "error" as const;
  if (status === "active") return "active" as const;
  return "pending" as const;
}

function errorMessage(error: unknown) {
  if (typeof error === "object" && error && "message" in error) return String((error as { message: unknown }).message);
  return String(error);
}
