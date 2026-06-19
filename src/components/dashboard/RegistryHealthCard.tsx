import { useState } from "react";
import type { RegistryHealth } from "../../types";

export interface RegistryHealthCardProps {
  profileName?: string;
  registryUrl?: string;
  health?: RegistryHealth;
  disabled?: boolean;
  onRefresh?: () => Promise<void>;
}

export function RegistryHealthCard({ profileName, registryUrl, health, disabled, onRefresh }: RegistryHealthCardProps) {
  const [refreshing, setRefreshing] = useState(false);
  const canRefresh = Boolean(registryUrl && onRefresh && !disabled && !refreshing);
  const badgeClass = health?.reachable ? "badge-success" : registryUrl ? "badge-warning" : "badge-info";
  const badgeLabel = healthStatusLabel(health?.status ?? (registryUrl ? "not_checked" : "no_profile"));

  async function handleRefresh() {
    if (!canRefresh || !onRefresh) return;
    setRefreshing(true);
    try {
      await onRefresh();
    } finally {
      setRefreshing(false);
    }
  }

  return (
    <div className="card" data-testid="rm-registry-health-card">
      <div className="card-header">
        <div className="card-title">❤️ Registry 健康状态</div>
        <span className={`badge ${badgeClass}`} data-testid="rm-registry-health-badge">{badgeLabel}</span>
      </div>
      <div className="card-body">
        <div className="metric">
          <div className="metric-value">{profileName ?? "未选择 Registry"}</div>
          <div className="metric-label">{registryUrl ?? "选择一个配置以检查 /v2/ 健康状态。"}</div>
        </div>
        <div className="preflight-item" role="status" data-testid="rm-registry-health-status">
          <span className="text-secondary">{health?.message ?? "此配置尚未检查健康状态。"}</span>
        </div>
        {health?.checkedAt ? (
          <p className="text-secondary" style={{ fontSize: "var(--text-sm)" }}>
            上次检查：{new Date(health.checkedAt).toLocaleString()}
          </p>
        ) : null}
        <button
          type="button"
          className="btn btn-secondary btn-sm"
          onClick={() => void handleRefresh()}
          disabled={!canRefresh}
          data-testid="rm-refresh-health-button"
        >
          {refreshing ? "正在刷新…" : "刷新状态"}
        </button>
      </div>
    </div>
  );
}

function healthStatusLabel(status: string) {
  switch (status) {
    case "ok":
      return "正常";
    case "not_checked":
      return "未检查";
    case "no_profile":
      return "未选择配置";
    case "v2_unavailable":
      return "V2 不可用";
    case "registry_api_error":
      return "Registry API 错误";
    case "timeout":
      return "超时";
    default:
      return status;
  }
}
