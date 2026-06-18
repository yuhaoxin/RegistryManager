import { Repository } from "../../types";

export interface RepositoryCardProps {
  repository: Repository;
  onClick: () => void;
}

export function RepositoryCard({ repository, onClick }: RepositoryCardProps) {
  return (
    <button
      type="button"
      className="card card-interactive repository-card"
      data-testid="rm-repository-card"
      onClick={onClick}
      aria-label={`Open ${repository.name}`}
    >
      <div className="repository-card-name">{repository.name}</div>
      <div className="repository-card-meta">
        <span className="badge badge-info">{repository.tagCount} tag{repository.tagCount === 1 ? "" : "s"}</span>
        {repository.size ? <span>{repository.size}</span> : null}
      </div>
      {repository.lastUpdated ? (
        <div className="text-muted" style={{ fontSize: "var(--text-xs)" }}>
          Updated {repository.lastUpdated}
        </div>
      ) : null}
    </button>
  );
}
