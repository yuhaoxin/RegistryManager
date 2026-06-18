export interface GcConfirmDialogProps {
  open: boolean;
  containerName: string;
  onConfirm: () => void;
  onCancel: () => void;
}

export function GcConfirmDialog({ open, containerName, onConfirm, onCancel }: GcConfirmDialogProps) {
  if (!open) {
    return null;
  }

  return (
    <div role="dialog" aria-modal="true" aria-labelledby="gc-confirm-title" data-testid="rm-gc-confirm-dialog">
      <div className="drawer-backdrop" onClick={onCancel} aria-hidden="true" />
      <div
        className="card"
        style={{
          position: "fixed",
          top: "50%",
          left: "50%",
          transform: "translate(-50%, -50%)",
          zIndex: 60,
          width: "min(420px, 90vw)",
        }}
      >
        <div className="card-header">
          <div className="card-title" id="gc-confirm-title">⚠️ Run garbage collection?</div>
        </div>
        <div className="card-body">
          <p className="text-secondary">
            This will stop the selected registry container
            <strong> {containerName}</strong>, run a temporary GC container with the original
            storage mounts, then restart the registry. Make sure no pushes are in progress.
          </p>
          <p className="preflight-item warn">
            Downtime warning: the original local registry is stopped before GC. This is destructive
            local maintenance and must not run while the registry is accepting writes.
          </p>
          <p className="text-secondary" style={{ fontSize: "var(--text-sm)" }}>
            Only untagged blobs will be removed. This action is recorded in the audit log.
          </p>
        </div>
        <div className="flex gap-2 justify-between">
          <button type="button" className="btn btn-secondary" onClick={onCancel}>
            Cancel
          </button>
          <button
            type="button"
            className="btn btn-danger"
            data-testid="rm-run-gc-button"
            onClick={onConfirm}
          >
            Run GC now
          </button>
        </div>
      </div>
    </div>
  );
}
