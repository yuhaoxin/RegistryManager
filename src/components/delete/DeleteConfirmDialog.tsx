import { useEffect, useState } from "react";
import type { DeleteImpact, DeleteResult } from "../../types";
import { runTauriCommand } from "../../hooks/useTauriCommand";

export interface DeleteConfirmDialogProps {
  open: boolean;
  profileId?: string;
  repository: string;
  reference: string;
  fallbackDigest: string;
  onClose: () => void;
  onDeleted?: (result: DeleteResult) => void;
}

export function DeleteConfirmDialog({ open, profileId, repository, reference, fallbackDigest, onClose, onDeleted }: DeleteConfirmDialogProps) {
  const [impact, setImpact] = useState<DeleteImpact | undefined>();
  const [message, setMessage] = useState<string | undefined>();
  const [loading, setLoading] = useState(false);

  useEffect(() => {
    if (!open) return;
    setMessage(undefined);
    const fallback: DeleteImpact = {
      repository,
      reference,
      digest: fallbackDigest,
      digestSuffix: fallbackDigest.slice(-12),
      mediaType: "unknown",
      affectedTags: [reference],
      isMultiArch: false,
      warning: "Storage may not be released until server-side GC completes.",
    };
    if (!profileId) {
      setImpact(fallback);
      return;
    }
    void runTauriCommand<DeleteImpact>("get_delete_impact", { profileId, repository, reference })
      .then(setImpact)
      .catch((error) => {
        setImpact(fallback);
        setMessage(errorMessage(error));
      });
  }, [fallbackDigest, open, profileId, reference, repository]);

  if (!open) return null;

  const requiredSuffix = impact?.digestSuffix ?? fallbackDigest.slice(-12);
  const canDelete = Boolean(impact && !loading);

  async function submit() {
    if (!impact || !canDelete) return;
    setLoading(true);
    setMessage(undefined);
    try {
      const result = await runTauriCommand<DeleteResult>("delete_manifest", {
        profileId,
        repository,
        reference,
        confirmedDigestSuffix: requiredSuffix,
      });
      setMessage(`Delete recorded as ${result.status}; storage may not be released until server-side GC.`);
      onDeleted?.(result);
    } catch (error) {
      setMessage(errorMessage(error));
    } finally {
      setLoading(false);
    }
  }

  return (
    <div role="dialog" aria-modal="true" aria-labelledby="delete-confirm-title" data-testid="delete-confirm-dialog" style={{ position: "fixed", inset: 0, zIndex: 70 }}>
      <div className="drawer-backdrop" onClick={onClose} aria-hidden="true" />
      <div className="card" style={{ position: "fixed", top: "50%", left: "50%", transform: "translate(-50%, -50%)", zIndex: 70, width: "min(680px, 92vw)" }}>
        <div className="card-header"><div className="card-title" id="delete-confirm-title">⚠️ Confirm delete</div></div>
        <div className="card-body">
          <p>
            Delete this image tag from <strong>{repository}</strong>?
          </p>
          <p className="text-secondary">
            This removes the resolved manifest digest. Storage is reclaimed only after registry GC.
          </p>
          {!impact ? <div role="status">Resolving manifest…</div> : null}
          {message ? <div role="status" className={message.toLowerCase().includes("failed") || message.includes("not found") ? "preflight-item error" : "preflight-item ok"}>{message}</div> : null}
        </div>
        <div className="flex gap-2 justify-between">
          <button type="button" className="btn btn-secondary" onClick={onClose}>Cancel</button>
          <button type="button" className="btn btn-danger" disabled={!canDelete} onClick={submit}>{loading ? "Deleting…" : "Confirm"}</button>
        </div>
      </div>
    </div>
  );
}

function errorMessage(error: unknown) {
  if (typeof error === "object" && error && "message" in error) return String((error as { message: unknown }).message);
  return String(error);
}
