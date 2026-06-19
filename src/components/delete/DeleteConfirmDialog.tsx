import { useEffect, useState } from "react";
import type { DeleteImpact, DeleteRepositoryResult, DeleteResult, ManifestSummary, RepositoryDeleteImpact, TagsPage } from "../../types";
import { runTauriCommand } from "../../hooks/useTauriCommand";
import { DeleteImpactList } from "./DeleteImpactList";

interface BaseDeleteConfirmDialogProps {
  open: boolean;
  profileId?: string;
  repository: string;
  onClose: () => void;
  onDeleted?: (result?: DeleteResult | DeleteRepositoryResult) => void;
  onAuditEventRecorded?: () => void;
}

export interface TagDeleteConfirmDialogProps extends BaseDeleteConfirmDialogProps {
  mode?: "tag";
  reference: string;
  fallbackDigest: string;
}

export interface RepositoryDeleteConfirmDialogProps extends BaseDeleteConfirmDialogProps {
  mode: "repository";
  registryUrl: string;
  tagCount: number;
  onDeleteRepository?: (repository: string) => Promise<void>;
}

export type DeleteConfirmDialogProps = TagDeleteConfirmDialogProps | RepositoryDeleteConfirmDialogProps;

export function DeleteConfirmDialog(props: DeleteConfirmDialogProps) {
  const [impact, setImpact] = useState<DeleteImpact | RepositoryDeleteImpact | undefined>();
  const [message, setMessage] = useState<string | undefined>();
  const [loading, setLoading] = useState(false);

  const mode = props.mode === "repository" ? "repository" : "tag";
  const { open, profileId, repository, onClose, onDeleted, onAuditEventRecorded } = props;

  let reference = "";
  let fallbackDigest = "";
  let tagCount = 0;
  let onDeleteRepository: ((repository: string) => Promise<void>) | undefined;
  if (mode === "tag") {
    const tagProps = props as TagDeleteConfirmDialogProps;
    reference = tagProps.reference;
    fallbackDigest = tagProps.fallbackDigest;
  } else {
    const repoProps = props as RepositoryDeleteConfirmDialogProps;
    tagCount = repoProps.tagCount;
    onDeleteRepository = repoProps.onDeleteRepository;
  }

  useEffect(() => {
    if (!open || mode !== "tag") return;
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
  }, [mode, open, profileId, repository, reference, fallbackDigest]);

  useEffect(() => {
    if (!open || mode !== "repository") return;
    setMessage(undefined);
    const fallback: RepositoryDeleteImpact = {
      repository,
      totalTags: tagCount,
      uniqueDigests: tagCount,
      affectedTags: [],
      warning: "Storage may not be released until server-side GC completes.",
    };
    if (!profileId) {
      setImpact(fallback);
      return;
    }
    void fetchRepositoryDeleteImpact(profileId, repository)
      .then(setImpact)
      .catch((error) => {
        setImpact(fallback);
        setMessage(errorMessage(error));
      });
  }, [mode, open, profileId, repository, tagCount]);

  if (!open) return null;

  const canDelete = Boolean(impact && !loading);
  const confirmLabel = mode === "repository" ? "Delete repository" : "Confirm";

  async function submit() {
    if (!impact || !canDelete) return;
    setLoading(true);
    setMessage(undefined);
    try {
      if (mode === "tag") {
        const requiredSuffix = (impact as DeleteImpact).digestSuffix;
        const result = await runTauriCommand<DeleteResult>("delete_manifest", {
          profileId,
          repository,
          reference,
          confirmedDigestSuffix: requiredSuffix,
        });
        onAuditEventRecorded?.();
        setMessage(`Delete recorded as ${result.status}; storage may not be released until server-side GC.`);
        onDeleted?.(result);
      } else {
        if (onDeleteRepository) {
          await onDeleteRepository(repository);
        } else {
          await runTauriCommand<DeleteRepositoryResult>("delete_repository", { profileId, repository });
        }
        onAuditEventRecorded?.();
        setMessage("Repository delete recorded; storage may not be released until server-side GC.");
        onDeleted?.();
      }
    } catch (error) {
      onAuditEventRecorded?.();
      setMessage(errorMessage(error));
    } finally {
      setLoading(false);
    }
  }

  return (
    <div role="dialog" aria-modal="true" aria-labelledby="delete-confirm-title" data-testid="delete-confirm-dialog" style={{ position: "fixed", inset: 0, zIndex: 70 }}>
      <div className="drawer-backdrop" onClick={onClose} aria-hidden="true" />
      <div className="card" style={{ position: "fixed", top: "50%", left: "50%", transform: "translate(-50%, -50%)", zIndex: 70, width: "min(680px, 92vw)" }}>
        <div className="card-header"><div className="card-title" id="delete-confirm-title">{mode === "repository" ? "⚠️ Confirm repository delete" : "⚠️ Confirm delete"}</div></div>
        <div className="card-body">
          {mode === "repository" ? (
            <>
              <p>
                Delete all tags and manifests in <strong>{repository}</strong>?
              </p>
              <p className="text-secondary">
                This removes every resolved manifest digest in the repository. Storage is reclaimed only after registry GC.
              </p>
            </>
          ) : (
            <>
              <p>
                Delete this image tag from <strong>{repository}</strong>?
              </p>
              <p className="text-secondary">
                This removes the resolved manifest digest. Storage is reclaimed only after registry GC.
              </p>
            </>
          )}
          {!impact ? <div role="status">Resolving impact…</div> : <DeleteImpactList impact={impact} />}
          {message ? <div role="status" className={message.toLowerCase().includes("failed") || message.includes("not found") ? "preflight-item error" : "preflight-item ok"}>{message}</div> : null}
        </div>
        <div className="flex gap-2 justify-between">
          <button type="button" className="btn btn-secondary" onClick={onClose}>Cancel</button>
          <button type="button" className="btn btn-danger" disabled={!canDelete} onClick={submit}>{loading ? "Deleting…" : confirmLabel}</button>
        </div>
      </div>
    </div>
  );
}

async function fetchRepositoryDeleteImpact(profileId: string, repository: string): Promise<RepositoryDeleteImpact> {
  const page = await runTauriCommand<TagsPage>("list_tags", { profileId, repository, n: 1000 });
  const tags = page.tags;
  const digests = new Set<string>();
  const affectedTags: string[] = [];
  await Promise.all(
    tags.map(async (tag) => {
      try {
        const summary = await runTauriCommand<ManifestSummary>("get_manifest", { profileId, repository, reference: tag.tag });
        digests.add(summary.digest);
        affectedTags.push(tag.tag);
      } catch {
        affectedTags.push(tag.tag);
      }
    })
  );
  return {
    repository,
    totalTags: tags.length,
    uniqueDigests: digests.size || affectedTags.length,
    affectedTags,
    warning: "Storage may not be released until server-side GC completes.",
  };
}

function errorMessage(error: unknown) {
  if (typeof error === "object" && error && "message" in error) return String((error as { message: unknown }).message);
  return String(error);
}
