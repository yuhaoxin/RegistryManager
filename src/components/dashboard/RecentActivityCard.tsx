import { AuditEvent } from "../../types";

export interface RecentActivityCardProps {
  events: AuditEvent[];
}

export function RecentActivityCard({ events }: RecentActivityCardProps) {
  return (
    <div className="card" data-testid="rm-recent-activity-card">
      <div className="card-header">
        <div className="card-title">📋 Recent activity</div>
      </div>
      <div className="card-body">
        {events.length === 0 ? (
          <p className="text-secondary">No recent activity.</p>
        ) : (
          <ul className="preflight-list">
            {events.map((event) => (
              <li key={event.id} className="preflight-item">
                <span
                  className={`badge badge-${
                    event.status === "success" ? "success" : event.status === "failure" ? "danger" : "warning"
                  }`}
                >
                  {event.status}
                </span>
                <span style={{ fontSize: "var(--text-sm)" }}>
                  {event.action}
                  {event.repository ? ` • ${event.repository}` : null}
                  {event.tag ? `:${event.tag}` : null}
                </span>
                <span className="text-muted" style={{ marginLeft: "auto", fontSize: "var(--text-xs)" }}>
                  {event.timestamp}
                </span>
              </li>
            ))}
          </ul>
        )}
      </div>
    </div>
  );
}
