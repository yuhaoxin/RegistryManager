import { AuditEvent } from "../../types";

export interface RecentActivityCardProps {
  events: AuditEvent[];
}

export function RecentActivityCard({ events }: RecentActivityCardProps) {
  return (
    <div className="card" data-testid="rm-recent-activity-card">
      <div className="card-header">
        <div className="card-title">📋 最近活动</div>
      </div>
      <div className="card-body">
        {events.length === 0 ? (
          <p className="text-secondary">暂无最近活动。</p>
        ) : (
          <ul className="preflight-list">
            {events.map((event) => (
              <li key={event.id} className="preflight-item">
                <span
                  className={`badge badge-${
                    event.status === "success" ? "success" : event.status === "failure" ? "danger" : "warning"
                  }`}
                >
                  {statusLabel(event.status)}
                </span>
                <span style={{ fontSize: "var(--text-sm)" }}>
                  {actionLabel(event.action)}
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
    default:
      return status;
  }
}

function actionLabel(action: string) {
  switch (action) {
    case "delete_manifest":
      return "删除清单";
    case "local_gc":
      return "本地 GC";
    default:
      return action;
  }
}
