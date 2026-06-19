import type { Repository } from "../../types";
import { isLocalRegistryUrl } from "../../utils/registryUrl";

export interface RepositoryCardProps {
  repository: Repository;
  registryUrl?: string;
  profileId?: string;
  onClick: () => void;
  onDeleteRequest?: (repository: Repository) => void;
}

export function RepositoryCard({ repository, registryUrl, profileId, onClick, onDeleteRequest }: RepositoryCardProps) {
  const isLocal = registryUrl ? isLocalRegistryUrl(registryUrl) : false;
  const canDelete = isLocal && Boolean(profileId) && repository.tagCount > 0;

  return (
    <div
      role="button"
      tabIndex={0}
      className="card card-interactive repository-card"
      data-testid="rm-repository-card"
      onClick={onClick}
      onKeyDown={(event) => {
        if (event.key === "Enter" || event.key === " ") {
          event.preventDefault();
          onClick();
        }
      }}
      aria-label={`Open ${repository.name}`}
    >
      <div className="repository-card-name">
        {repository.name}
        {repository.stale ? (
          <span className="badge badge-warning" data-testid="rm-repository-stale-marker">
            Stale cache
          </span>
        ) : null}
      </div>
      <div className="repository-card-meta">
        <span className="badge badge-info">{repository.tagCount} tag{repository.tagCount === 1 ? "" : "s"}</span>
        {repository.size ? <span>{repository.size}</span> : null}
      </div>
      {repository.lastUpdated ? (
        <div className="text-muted" style={{ fontSize: "var(--text-xs)" }}>
          Updated {repository.lastUpdated}
        </div>
      ) : null}
      <div className="flex gap-2 items-center">
        {canDelete ? (
          <button
            type="button"
            className="btn btn-danger btn-sm"
            data-testid="rm-repository-delete-button"
            onClick={(event) => {
              event.stopPropagation();
              onDeleteRequest?.(repository);
            }}
          >
            Delete
          </button>
        ) : (
          <span className="text-muted" style={{ fontSize: "var(--text-xs)" }} data-testid="rm-repository-delete-disabled">
            {isLocal ? "No tags to delete" : "Remote delete disabled — destructive operations require a local registry"}
          </span>
        )}
      </div>
    </div>
  );
}
