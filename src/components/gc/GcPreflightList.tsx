export interface GcPreflightItem {
  name: string;
  status: "ok" | "warn" | "error";
  message: string;
}

export interface GcPreflightListProps {
  items: GcPreflightItem[];
}

export function GcPreflightList({ items }: GcPreflightListProps) {
  return (
    <div data-testid="rm-gc-preflight-list">
      <div className="gc-section-title">Preflight checks</div>
      <ul className="preflight-list" role="list">
        {items.map((item) => (
          <li key={item.name} className={`preflight-item ${item.status}`}>
            <span aria-hidden="true">{item.status === "ok" ? "✓" : item.status === "warn" ? "⚠" : "✕"}</span>
            <div>
              <div style={{ fontWeight: 600 }}>{item.name}</div>
              <div style={{ fontSize: "var(--text-xs)", opacity: 0.8 }}>{item.message}</div>
            </div>
          </li>
        ))}
      </ul>
    </div>
  );
}
