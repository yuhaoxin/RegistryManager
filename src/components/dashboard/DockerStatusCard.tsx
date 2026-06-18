import { DockerStatus } from "../../types";

export interface DockerStatusCardProps {
  status: DockerStatus;
}

export function DockerStatusCard({ status }: DockerStatusCardProps) {
  const reachable = status.reachable ?? status.available ?? false;

  return (
    <div className="card" data-testid="rm-docker-status-card">
      <div className="card-header">
        <div className="card-title">
          🐳 Docker
          <span className="sr-only">Status</span>
        </div>
        {reachable ? (
          <span className="badge badge-success">Connected</span>
        ) : (
          <span className="badge badge-danger">Unavailable</span>
        )}
      </div>
      <div className="card-body">
        {reachable ? (
          <>
            <div className="metric">
              <div className="metric-value">{status.version ?? "Unknown"}</div>
              <div className="metric-label">Engine version</div>
            </div>
            <div className="metric">
              <div className="metric-value text-secondary">{status.context ?? "default"}</div>
              <div className="metric-label">Context</div>
            </div>
          </>
        ) : (
          <div data-testid="rm-docker-unavailable-empty" role="status">
            <p className="text-secondary">{status.error ?? "Docker daemon is not reachable."}</p>
          </div>
        )}
      </div>
    </div>
  );
}
