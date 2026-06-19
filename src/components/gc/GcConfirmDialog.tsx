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
          <div className="card-title" id="gc-confirm-title">⚠️ 运行垃圾回收？</div>
        </div>
        <div className="card-body">
          <p className="text-secondary">
            这会停止所选 Registry 容器
            <strong> {containerName}</strong>，使用原始存储挂载运行临时 GC 容器，随后重启 Registry。请确保当前没有推送正在进行。
          </p>
          <p className="preflight-item warn">
            停机警告：原本地 Registry 会在 GC 前停止。这是破坏性的本地维护操作，不得在 Registry 接受写入时运行。
          </p>
          <p className="text-secondary" style={{ fontSize: "var(--text-sm)" }}>
            只会移除未打标签的 blob。此操作会记录到审计日志。
          </p>
        </div>
        <div className="flex gap-2 justify-between">
          <button type="button" className="btn btn-secondary" onClick={onCancel}>
            取消
          </button>
          <button
            type="button"
            className="btn btn-danger"
            data-testid="rm-run-gc-button"
            onClick={onConfirm}
          >
            立即运行 GC
          </button>
        </div>
      </div>
    </div>
  );
}
