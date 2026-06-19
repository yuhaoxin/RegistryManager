export interface RepositorySearchProps {
  value: string;
  onChange: (value: string) => void;
  placeholder?: string;
}

export function RepositorySearch({ value, onChange, placeholder = "搜索仓库…" }: RepositorySearchProps) {
  return (
    <div className="search-bar" data-testid="rm-repository-search">
      <span className="search-bar-icon" aria-hidden="true">🔍</span>
      <input
        type="search"
        className="input"
        value={value}
        onChange={(e) => onChange(e.target.value)}
        placeholder={placeholder}
        aria-label="搜索仓库"
      />
    </div>
  );
}
