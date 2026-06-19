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
  const badgeLabel = health?.status ?? (registryUrl ? "not_checked" : "no_profile");

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
        <div className="card-title">❤️ Registry health</div>
        <span className={`badge ${badgeClass}`} data-testid="rm-registry-health-badge">{badgeLabel}</span>
      </div>
      <div className="card-body">
        <div className="metric">
          <div className="metric-value">{profileName ?? "No registry selected"}</div>
          <div className="metric-label">{registryUrl ?? "Select a profile to check /v2/ health."}</div>
        </div>
        <div className="preflight-item" role="status" data-testid="rm-registry-health-status">
          <span className="text-secondary">{health?.message ?? "Health has not been checked for this profile yet."}</span>
        </div>
        {health?.checkedAt ? (
          <p className="text-secondary" style={{ fontSize: "var(--text-sm)" }}>
            Last checked: {new Date(health.checkedAt).toLocaleString()}
          </p>
        ) : null}
        <button
          type="button"
          className="btn btn-secondary btn-sm"
          onClick={() => void handleRefresh()}
          disabled={!canRefresh}
          data-testid="rm-refresh-health-button"
        >
          {refreshing ? "Refreshing…" : "Refresh status"}
        </button>
      </div>
    </div>
  );
}
