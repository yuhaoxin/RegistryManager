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
    title: stepTitle(item.id),
    status: mapStepStatus(item.status),
    note: item.message,
  })) ?? [];

  const logs = result?.logs ?? [];

  if (!gcAvailable) return null;

  return (
    <div className="card" data-testid="rm-local-gc-executor">
      <div className="card-header">
        <div className="card-title">🧹 本地 GC 执行器</div>
        <button type="button" className="btn btn-danger btn-sm" onClick={() => setShowConfirm(true)} disabled={running}>
          {running ? "正在运行 GC…" : "运行 GC"}
        </button>
      </div>
      <div className="card-body">
        <GcPreflightList items={[]} />
        <GcStepTimeline steps={steps} />
        <GcLiveLogPanel logs={logs} />
        {error ? <div className="preflight-item error" role="alert">{error}</div> : null}
        {result?.status === "gc_failed" ? (
          <div className="preflight-item error" role="alert">
            需要恢复：{result.recoveryAction}
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
        containerName={linkedContainerLabel ?? "所选本地 Registry"}
        onConfirm={runGc}
        onCancel={() => setShowConfirm(false)}
      />
    </div>
  );
}

function stepTitle(id: string) {
  switch (id) {
    case "snapshot":
      return "快照";
    case "preflight":
      return "预检";
    case "stop":
      return "停止容器";
    case "gc":
      return "执行 GC";
    case "cleanup":
      return "清理";
    case "restart":
      return "重启容器";
    case "health":
      return "健康检查";
    case "failure_recovery":
      return "失败恢复";
    default:
      return id;
  }
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
