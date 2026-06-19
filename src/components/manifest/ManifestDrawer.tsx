import { useState } from "react";
import { StaleCacheBanner } from "../common";
import { DeleteConfirmDialog } from "../delete/DeleteConfirmDialog";
import { Manifest } from "../../types";

export interface ManifestDrawerProps {
  open: boolean;
  repositoryName: string;
  manifest?: Manifest;
  profileId?: string;
  onClose: () => void;
  onDeleted?: () => void;
  onAuditEventRecorded?: () => void;
}

export function ManifestDrawer({ open, repositoryName, manifest, profileId, onClose, onDeleted, onAuditEventRecorded }: ManifestDrawerProps) {
  const [deleteOpen, setDeleteOpen] = useState(false);
  if (!open) {
    return null;
  }

  return (
    <div className="drawer-shell" role="dialog" aria-modal="true" aria-labelledby="manifest-title" data-testid="rm-manifest-drawer">
      <div className="drawer-backdrop" onClick={onClose} aria-hidden="true" />
      <div className="drawer">
        <div className="drawer-header">
          <div>
            <div className="card-title" id="manifest-title">📄 清单详情</div>
            <div className="card-subtitle">{repositoryName}</div>
          </div>
          <button type="button" className="btn btn-ghost" onClick={onClose} aria-label="关闭清单抽屉">
            ✕
          </button>
        </div>

        <div className="drawer-body">
          {manifest ? (
            <>
              {manifest.stale ? <StaleCacheBanner message="Registry 离线。正在显示已过期的缓存清单。" /> : null}
              <div className="card" style={{ background: "var(--color-bg-sunken)" }}>
                <div className="card-body">
                  <div className="metric">
                    <div className="metric-label">摘要</div>
                    <div className="metric-value" style={{ fontSize: "var(--text-sm)", wordBreak: "break-all" }}>
                      {manifest.digest}
                    </div>
                  </div>
                  <div className="flex gap-4">
                    <div className="metric">
                      <div className="metric-label">媒体类型</div>
                      <div className="metric-value" style={{ fontSize: "var(--text-sm)" }}>{manifest.mediaType}</div>
                    </div>
                    <div className="metric">
                      <div className="metric-label">大小</div>
                      <div className="metric-value" style={{ fontSize: "var(--text-sm)" }}>{manifest.size} 字节</div>
                    </div>
                  </div>
                  {manifest.platform ? (
                    <div className="metric">
                      <div className="metric-label">平台</div>
                      <div className="metric-value" style={{ fontSize: "var(--text-sm)" }}>{manifest.platform}</div>
                    </div>
                  ) : null}
                  {manifest.layers?.length ? (
                    <div className="metric">
                      <div className="metric-label">层</div>
                      <div className="metric-value" style={{ fontSize: "var(--text-sm)" }}>{manifest.layers.length}</div>
                    </div>
                  ) : null}
                  {manifest.platforms?.length ? (
                    <div className="metric">
                      <div className="metric-label">平台</div>
                      <div className="metric-value" style={{ fontSize: "var(--text-sm)" }}>
                        {manifest.platforms.map((platform) => `${platform.os ?? "unknown"}/${platform.architecture ?? "unknown"}`).join(", ")}
                      </div>
                    </div>
                  ) : null}
                </div>
              </div>

              {manifest.layers?.length ? (
                <div className="gc-section">
                  <div className="gc-section-title">层</div>
                  <ul className="preflight-list">
                    {manifest.layers.map((layer) => (
                      <li className="preflight-item" key={layer.digest}>
                        <span className="font-mono">{layer.digest}</span>
                        <span className="text-muted" style={{ marginLeft: "auto" }}>{layer.size} 字节</span>
                      </li>
                    ))}
                  </ul>
                </div>
              ) : null}

              <div className="gc-section">
                <div className="gc-section-title">原始清单 JSON</div>
                <pre className="log-panel" style={{ maxHeight: "320px" }}>{manifest.rawJson}</pre>
              </div>
            </>
          ) : (
            <div className="card" style={{ background: "var(--color-bg-sunken)" }}>
              <div className="card-body">
                <p className="text-secondary" data-testid="rm-manifest-empty">未选择清单。</p>
              </div>
            </div>
          )}
        </div>

        <div className="drawer-footer">
          <button type="button" className="btn btn-secondary" onClick={onClose}>
            关闭
          </button>
          <button type="button" className="btn btn-danger" data-testid="delete-manifest-button" onClick={() => setDeleteOpen(true)}>
            删除标签
          </button>
        </div>
      </div>
      <DeleteConfirmDialog
        open={deleteOpen}
        profileId={profileId}
        repository={repositoryName}
        reference={manifest?.digest ?? ""}
        fallbackDigest={manifest?.digest ?? ""}
        onClose={() => setDeleteOpen(false)}
        onAuditEventRecorded={onAuditEventRecorded}
        onDeleted={() => {
          setDeleteOpen(false);
          onClose();
          onDeleted?.();
        }}
      />
    </div>
  );
}
