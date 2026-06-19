import { useState } from "react";
import type { Repository } from "../../types";
import { DeleteConfirmDialog } from "../delete/DeleteConfirmDialog";
import { StaleCacheBanner } from "../common";
import { RepositoryCard } from "./RepositoryCard";
import { RepositorySearch } from "./RepositorySearch";

export interface RepositoryBrowserProps {
  repositories: Repository[];
  search: string;
  stale?: boolean;
  nextCursor?: string;
  profileId?: string;
  registryUrl?: string;
  onSearchChange: (value: string) => void;
  onRepositorySelect: (repository: Repository) => void;
  onRepositoryDelete?: (repository: string) => Promise<void>;
  onAuditEventRecorded?: () => void;
  onLoadMore?: () => void;
}

export function RepositoryBrowser({
  repositories,
  search,
  stale,
  nextCursor,
  profileId,
  registryUrl,
  onSearchChange,
  onRepositorySelect,
  onRepositoryDelete,
  onAuditEventRecorded,
  onLoadMore,
}: RepositoryBrowserProps) {
  const term = search.trim().toLowerCase();
  const filteredRepositories = term
    ? repositories.filter((repo) => repo.name.toLowerCase().includes(term))
    : repositories;

  const [deleteTarget, setDeleteTarget] = useState<Repository | undefined>(undefined);

  return (
    <section className="card" aria-label="Repository browser" data-testid="rm-repository-browser">
      <div className="card-header">
        <div className="card-title">📚 Repositories</div>
        {nextCursor ? <button className="btn btn-secondary btn-sm" onClick={onLoadMore} type="button">Load more</button> : null}
      </div>
      <div className="card-body">
        {stale ? <StaleCacheBanner /> : null}
        <RepositorySearch value={search} onChange={onSearchChange} />
        <div className="repository-grid">
          {filteredRepositories.map((repo) => {
            const repository = stale ? { ...repo, stale: true } : repo;
            return (
              <RepositoryCard
                key={repo.name}
                repository={repository}
                registryUrl={registryUrl}
                profileId={profileId}
                onClick={() => onRepositorySelect(repo)}
                onDeleteRequest={setDeleteTarget}
              />
            );
          })}
        </div>
        {filteredRepositories.length === 0 ? (
          <div className="state-center" data-testid="no-search-results" role="status">
            <div className="state-title">No repositories match "{search}".</div>
          </div>
        ) : null}
      </div>

      {deleteTarget && registryUrl ? (
        <DeleteConfirmDialog
          open={Boolean(deleteTarget)}
          mode="repository"
          profileId={profileId}
          registryUrl={registryUrl}
          repository={deleteTarget.name}
          tagCount={deleteTarget.tagCount}
          onClose={() => setDeleteTarget(undefined)}
          onAuditEventRecorded={onAuditEventRecorded}
          onDeleteRepository={async (repository) => {
            await onRepositoryDelete?.(repository);
            setDeleteTarget(undefined);
          }}
        />
      ) : null}
    </section>
  );
}
