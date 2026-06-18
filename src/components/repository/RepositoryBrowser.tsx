import { Repository } from "../../types";
import { StaleCacheBanner } from "../common";
import { RepositoryCard } from "./RepositoryCard";
import { RepositorySearch } from "./RepositorySearch";

export interface RepositoryBrowserProps {
  repositories: Repository[];
  search: string;
  stale?: boolean;
  nextCursor?: string;
  onSearchChange: (value: string) => void;
  onRepositorySelect: (repository: Repository) => void;
  onLoadMore?: () => void;
}

export function RepositoryBrowser({ repositories, search, stale, nextCursor, onSearchChange, onRepositorySelect, onLoadMore }: RepositoryBrowserProps) {
  const term = search.trim().toLowerCase();
  const filteredRepositories = term
    ? repositories.filter((repo) => repo.name.toLowerCase().includes(term))
    : repositories;

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
          {filteredRepositories.map((repo) => (
            <RepositoryCard key={repo.name} repository={repo} onClick={() => onRepositorySelect(repo)} />
          ))}
        </div>
        {filteredRepositories.length === 0 ? (
          <div className="state-center" data-testid="no-search-results" role="status">
            <div className="state-title">No repositories match "{search}".</div>
          </div>
        ) : null}
      </div>
    </section>
  );
}
