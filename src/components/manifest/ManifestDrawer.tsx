import { useState } from "react";
import { DeleteConfirmDialog } from "../delete/DeleteConfirmDialog";
import { Manifest } from "../../types";

export interface ManifestDrawerProps {
  open: boolean;
  repositoryName: string;
  manifest: Manifest;
  profileId?: string;
  onClose: () => void;
  onDeleted?: () => void;
}

export function ManifestDrawer({ open, repositoryName, manifest, profileId, onClose, onDeleted }: ManifestDrawerProps) {
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
            <div className="card-title" id="manifest-title">📄 Manifest detail</div>
            <div className="card-subtitle">{repositoryName}</div>
          </div>
          <button type="button" className="btn btn-ghost" onClick={onClose} aria-label="Close manifest drawer">
            ✕
          </button>
        </div>

        <div className="drawer-body">
          <div className="card" style={{ background: "var(--color-bg-sunken)" }}>
            <div className="card-body">
              <div className="metric">
                <div className="metric-label">Digest</div>
                <div className="metric-value" style={{ fontSize: "var(--text-sm)", wordBreak: "break-all" }}>
                  {manifest.digest}
                </div>
              </div>
              <div className="flex gap-4">
                <div className="metric">
                  <div className="metric-label">Media type</div>
                  <div className="metric-value" style={{ fontSize: "var(--text-sm)" }}>{manifest.mediaType}</div>
                </div>
                <div className="metric">
                  <div className="metric-label">Size</div>
                  <div className="metric-value" style={{ fontSize: "var(--text-sm)" }}>{manifest.size} bytes</div>
                </div>
              </div>
              {manifest.platform ? (
                <div className="metric">
                  <div className="metric-label">Platform</div>
                  <div className="metric-value" style={{ fontSize: "var(--text-sm)" }}>{manifest.platform}</div>
                </div>
              ) : null}
              {manifest.layers?.length ? (
                <div className="metric">
                  <div className="metric-label">Layers</div>
                  <div className="metric-value" style={{ fontSize: "var(--text-sm)" }}>{manifest.layers.length}</div>
                </div>
              ) : null}
              {manifest.platforms?.length ? (
                <div className="metric">
                  <div className="metric-label">Platforms</div>
                  <div className="metric-value" style={{ fontSize: "var(--text-sm)" }}>
                    {manifest.platforms.map((platform) => `${platform.os ?? "unknown"}/${platform.architecture ?? "unknown"}`).join(", ")}
                  </div>
                </div>
              ) : null}
            </div>
          </div>

          {manifest.layers?.length ? (
            <div className="gc-section">
              <div className="gc-section-title">Layers</div>
              <ul className="preflight-list">
                {manifest.layers.map((layer) => (
                  <li className="preflight-item" key={layer.digest}>
                    <span className="font-mono">{layer.digest}</span>
                    <span className="text-muted" style={{ marginLeft: "auto" }}>{layer.size} bytes</span>
                  </li>
                ))}
              </ul>
            </div>
          ) : null}

          <div className="gc-section">
            <div className="gc-section-title">Raw manifest JSON</div>
            <pre className="log-panel" style={{ maxHeight: "320px" }}>{manifest.rawJson}</pre>
          </div>
        </div>

        <div className="drawer-footer">
          <button type="button" className="btn btn-secondary" onClick={onClose}>
            Close
          </button>
          <button type="button" className="btn btn-danger" data-testid="delete-manifest-button" onClick={() => setDeleteOpen(true)}>
            Delete tag
          </button>
        </div>
      </div>
      <DeleteConfirmDialog
        open={deleteOpen}
        profileId={profileId}
        repository={repositoryName}
        reference={manifest.digest}
        fallbackDigest={manifest.digest}
        onClose={() => setDeleteOpen(false)}
        onDeleted={() => {
          onDeleted?.();
        }}
      />
    </div>
  );
}
