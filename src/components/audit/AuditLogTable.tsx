import { useEffect, useState } from "react";
import type { AuditEvent } from "../../types";
import { runTauriCommand } from "../../hooks/useTauriCommand";

export function AuditLogTable() {
  const [events, setEvents] = useState<AuditEvent[]>([]);

  useEffect(() => {
    let active = true;
    async function load() {
      try {
        const rows = await runTauriCommand<AuditEvent[]>("list_audit_events", { limit: 25, offset: 0 });
        if (active) setEvents(rows);
      } catch {
        if (active) setEvents([]);
      }
    }
    void load();
    const timer = window.setInterval(load, 750);
    return () => {
      active = false;
      window.clearInterval(timer);
    };
  }, []);

  return (
    <div className="card" id="audit" data-testid="rm-audit-log-table">
      <div className="card-header"><div className="card-title">📜 Audit log</div></div>
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
}

function actionLabel(action: AuditEvent["action"]) {
  return action;
}
