import { useState } from "react";
import { runTauriCommand } from "../../hooks/useTauriCommand";
import type { GcResult } from "../../types";
import { isLocalRegistryUrl } from "../../utils/registryUrl";
import { GcConfirmDialog } from "./GcConfirmDialog";
import { GcLiveLogPanel } from "./GcLiveLogPanel";
import { GcPreflightList } from "./GcPreflightList";
import { GcResultSummary } from "./GcResultSummary";
import { GcStepTimeline } from "./GcStepTimeline";

export interface LocalGcExecutorProps {
  containerId?: string | null;
  containerName?: string | null;
  profileId?: string;
  registryUrl?: string;
  onAuditEventRecorded?: () => void;
}

export function LocalGcExecutor({ containerId, containerName, profileId, registryUrl, onAuditEventRecorded }: LocalGcExecutorProps) {
  const [showConfirm, setShowConfirm] = useState(false);
  const [result, setResult] = useState<GcResult | undefined>();
  const [error, setError] = useState<string | undefined>();
  const [running, setRunning] = useState(false);
  const linkedContainerLabel = containerName?.trim() || registryUrl || containerId?.slice(0, 12);
  const gcAvailable = Boolean(profileId && registryUrl && isLocalRegistryUrl(registryUrl));

  async function runGc() {
    setShowConfirm(false);
    setRunning(true);
    setError(undefined);
    try {
      const output = await runTauriCommand<GcResult>("run_local_gc", { profileId, confirmDowntime: true });
      setResult(output);
      onAuditEventRecorded?.();
    } catch (err) {
      setError(errorMessage(err));
      onAuditEventRecorded?.();
    } finally {
      setRunning(false);
    }
  }

  const steps = result?.steps.map((item) => ({
    id: item.id,
    title: item.id,
    status: mapStepStatus(item.status),
    note: item.message,
  })) ?? [];

  const logs = result?.logs ?? [];

  if (!gcAvailable) return null;

  return (
    <div className="card" data-testid="rm-local-gc-executor">
      <div className="card-header">
        <div className="card-title">🧹 Local GC executor</div>
        <button type="button" className="btn btn-danger btn-sm" onClick={() => setShowConfirm(true)} disabled={running}>
          {running ? "Running GC…" : "Run GC"}
        </button>
      </div>
      <div className="card-body">
        <GcPreflightList items={[]} />
        <GcStepTimeline steps={steps} />
        <GcLiveLogPanel logs={logs} />
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
        containerName={linkedContainerLabel ?? "selected local registry"}
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
