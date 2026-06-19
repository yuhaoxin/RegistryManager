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
          <span className="sr-only">状态</span>
        </div>
        {reachable ? (
          <span className="badge badge-success">已连接</span>
        ) : (
          <span className="badge badge-danger">不可用</span>
        )}
      </div>
      <div className="card-body">
        {reachable ? (
          <>
            <div className="metric">
              <div className="metric-value">{status.version ?? "未知"}</div>
              <div className="metric-label">引擎版本</div>
            </div>
            <div className="metric">
              <div className="metric-value text-secondary">{status.context ?? "default"}</div>
              <div className="metric-label">上下文</div>
            </div>
          </>
        ) : (
          <div data-testid="rm-docker-status-empty" role="status">
            <p className="text-secondary">{status.error ?? "无法连接 Docker 守护进程。"}</p>
          </div>
        )}
      </div>
    </div>
  );
}
