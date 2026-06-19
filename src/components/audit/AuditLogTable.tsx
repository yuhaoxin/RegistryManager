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
        <div className="card-title">📜 Audit log</div>
        <button type="button" className="btn btn-secondary btn-sm" data-testid="rm-refresh-audit-log-button" onClick={() => void refresh()}>
          Refresh audit log
        </button>
      </div>
      <div className="card-body">
        <table className="audit-table">
          <thead><tr><th>Time</th><th>Action</th><th>Repository</th><th>Digest</th><th>Status</th><th>Error</th></tr></thead>
          <tbody>
            {events.length ? events.map((event) => (
              <tr key={event.id}>
                <td>{new Date(event.timestamp).toLocaleString()}</td>
                <td>{actionLabel(event.action)}</td>
                <td>{event.repositoryName ?? event.repository ?? "—"}</td>
                <td className="font-mono">{event.digest ?? "—"}</td>
                <td>{event.status}</td>
                <td>{event.errorMessage ?? "—"}</td>
              </tr>
            )) : <tr><td colSpan={6}>No audit events yet.</td></tr>}
          </tbody>
        </table>
      </div>
    </div>
  );
});

function actionLabel(action: AuditEvent["action"]) {
  return action;
}
