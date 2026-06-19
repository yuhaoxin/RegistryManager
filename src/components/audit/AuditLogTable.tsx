import { forwardRef, useCallback, useEffect, useImperativeHandle, useRef, useState } from "react";
import type { AuditEvent } from "../../types";
import { runTauriCommand } from "../../hooks/useTauriCommand";

export interface AuditLogTableHandle {
  refresh: () => Promise<void>;
}

export const AuditLogTable = forwardRef<AuditLogTableHandle>(function AuditLogTable(_props, ref) {
  const [events, setEvents] = useState<AuditEvent[]>([]);
  const mountedRef = useRef(false);

  const refresh = useCallback(async () => {
    try {
      const rows = await runTauriCommand<AuditEvent[]>("list_audit_events", { limit: 25, offset: 0 });
      if (mountedRef.current) setEvents(rows);
    } catch {
      if (mountedRef.current) setEvents([]);
    }
  }, []);

  useImperativeHandle(ref, () => ({ refresh }), [refresh]);

  useEffect(() => {
    mountedRef.current = true;
    void refresh();
    return () => {
      mountedRef.current = false;
    };
  }, [refresh]);

  return (
    <div className="card" id="audit" data-testid="rm-audit-log-table">
      <div className="card-header">
        <div className="card-title">📜 审计日志</div>
        <button type="button" className="btn btn-secondary btn-sm" data-testid="rm-refresh-audit-log-button" onClick={() => void refresh()}>
          刷新审计日志
        </button>
      </div>
      <div className="card-body">
        <table className="audit-table">
          <thead><tr><th>时间</th><th>操作</th><th>仓库</th><th>摘要</th><th>状态</th><th>错误</th></tr></thead>
          <tbody>
            {events.length ? events.map((event) => (
              <tr key={event.id}>
                <td>{new Date(event.timestamp).toLocaleString()}</td>
                <td>{actionLabel(event.action)}</td>
                <td>{event.repositoryName ?? event.repository ?? "—"}</td>
                <td className="font-mono">{event.digest ?? "—"}</td>
                <td>{statusLabel(event.status)}</td>
                <td>{event.errorMessage ?? "—"}</td>
              </tr>
            )) : <tr><td colSpan={6}>暂无审计事件。</td></tr>}
          </tbody>
        </table>
      </div>
    </div>
  );
});

function actionLabel(action: AuditEvent["action"]) {
  switch (action) {
    case "delete_manifest":
      return "删除清单";
    case "local_gc":
      return "本地 GC";
    default:
      return action;
  }
}

function statusLabel(status: string) {
  switch (status) {
    case "success":
      return "成功";
    case "failure":
      return "失败";
    case "pending_gc":
      return "等待 GC";
    case "gc_completed":
      return "GC 已完成";
    case "gc_failed":
      return "GC 失败";
    case "partial_failure":
      return "部分失败";
    default:
      return status;
  }
}
