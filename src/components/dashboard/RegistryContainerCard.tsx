import { RegistryContainer, RegistryHealth } from "../../types";

export interface RegistryContainerCardProps {
  container?: RegistryContainer;
  health?: RegistryHealth;
}

export function RegistryContainerCard({ container, health }: RegistryContainerCardProps) {
  if (!container) {
    return null;
  }

  const statusBadge =
    container.status === "running" ? (
      <span className="badge badge-success">{container.status}</span>
    ) : container.status === "exited" ? (
      <span className="badge badge-danger">{container.status}</span>
    ) : (
      <span className="badge badge-warning">{container.status}</span>
    );

  return (
    <div className="card" data-testid="rm-registry-container-card">
      <div className="card-header">
        <div className="card-title">
          📦 Registry container
          <span className="sr-only">{container.name}</span>
        </div>
        {statusBadge}
      </div>
      <div className="card-body">
        <div className="metric">
          <div className="metric-value">{container.image}</div>
          <div className="metric-label">Container: {container.name}</div>
        </div>
        <div className="flex gap-4">
          <div className="metric">
            <div className="metric-value text-secondary">{container.ports.join(", ") || "—"}</div>
            <div className="metric-label">Ports</div>
          </div>
          <div className="metric">
            <div className="metric-value text-secondary">{container.createdAt}</div>
            <div className="metric-label">Created</div>
          </div>
        </div>
        {health ? (
          <div className="preflight-item" data-testid="rm-registry-health-status">
            <span className={`badge ${health.reachable ? "badge-success" : "badge-warning"}`}>{health.status}</span>
            <span className="text-secondary">{health.message}</span>
          </div>
        ) : null}
      </div>
    </div>
  );
}
