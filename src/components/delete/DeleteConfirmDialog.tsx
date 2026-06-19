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
      warning: "在服务端 GC 完成前，存储空间可能不会释放。",
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
      warning: "在服务端 GC 完成前，存储空间可能不会释放。",
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
  const confirmLabel = mode === "repository" ? "删除仓库" : "确认";

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
        setMessage(`删除已记录为 ${statusLabel(result.status)}；在服务端 GC 完成前，存储空间可能不会释放。`);
        onDeleted?.(result);
      } else {
        if (onDeleteRepository) {
          await onDeleteRepository(repository);
        } else {
          await runTauriCommand<DeleteRepositoryResult>("delete_repository", { profileId, repository });
        }
        onAuditEventRecorded?.();
        setMessage("仓库删除已记录；在服务端 GC 完成前，存储空间可能不会释放。");
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
        <div className="card-header"><div className="card-title" id="delete-confirm-title">{mode === "repository" ? "⚠️ 确认删除仓库" : "⚠️ 确认删除"}</div></div>
        <div className="card-body">
          {mode === "repository" ? (
            <>
              <p>
                删除 <strong>{repository}</strong> 中的所有标签和清单？
              </p>
              <p className="text-secondary">
                这会移除仓库中所有已解析的清单摘要。只有在 Registry GC 后才会回收存储空间。
              </p>
            </>
          ) : (
            <>
              <p>
                从 <strong>{repository}</strong> 删除此镜像标签？
              </p>
              <p className="text-secondary">
                这会移除已解析的清单摘要。只有在 Registry GC 后才会回收存储空间。
              </p>
            </>
          )}
          {!impact ? <div role="status">正在解析影响范围…</div> : <DeleteImpactList impact={impact} />}
          {message ? <div role="status" className={message.toLowerCase().includes("failed") || message.includes("not found") || message.includes("失败") || message.includes("未找到") ? "preflight-item error" : "preflight-item ok"}>{message}</div> : null}
        </div>
        <div className="flex gap-2 justify-between">
          <button type="button" className="btn btn-secondary" onClick={onClose}>取消</button>
          <button type="button" className="btn btn-danger" disabled={!canDelete} onClick={submit}>{loading ? "正在删除…" : confirmLabel}</button>
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
    warning: "在服务端 GC 完成前，存储空间可能不会释放。",
  };
}

function statusLabel(status: string) {
  switch (status) {
    case "pending_gc":
      return "等待 GC";
    case "success":
      return "成功";
    case "failure":
      return "失败";
    case "partial_failure":
      return "部分失败";
    default:
      return status;
  }
}

function errorMessage(error: unknown) {
  if (typeof error === "object" && error && "message" in error) return String((error as { message: unknown }).message);
  return String(error);
}
