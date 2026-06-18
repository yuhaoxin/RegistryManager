export interface StorageReclaimCardProps {
  reclaimableBytes?: number;
}

function formatBytes(bytes: number): string {
  if (bytes === 0) return "0 B";
  const units = ["B", "KB", "MB", "GB", "TB"];
  const i = Math.floor(Math.log(bytes) / Math.log(1024));
  return `${(bytes / 1024 ** i).toFixed(2)} ${units[i]}`;
}

export function StorageReclaimCard({ reclaimableBytes = 0 }: StorageReclaimCardProps) {
  return (
    <div className="card" data-testid="rm-storage-reclaim-card">
      <div className="card-header">
        <div className="card-title">♻️ Storage reclaim</div>
      </div>
      <div className="card-body">
        <div className="metric">
          <div className="metric-value">{formatBytes(reclaimableBytes)}</div>
          <div className="metric-label">Estimated reclaimable space</div>
        </div>
        <p className="text-secondary" style={{ fontSize: "var(--text-sm)" }}>
          Run a local garbage-collection cycle against the selected registry container. The original
          container state is preserved.
        </p>
      </div>
    </div>
  );
}
