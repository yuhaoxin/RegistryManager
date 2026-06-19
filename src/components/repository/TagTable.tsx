import { Tag } from "../../types";

export interface TagTableProps {
  tags: Tag[];
  stale?: boolean;
  onSelect?: (tag: Tag) => void;
}

export function TagTable({ tags, stale, onSelect }: TagTableProps) {
  return (
    <div className="card" data-testid="rm-tag-table">
      <div className="card-header">
        <div className="card-title">🏷️ 标签</div>
      </div>
      <div className="card-body">
        <table className="data-table">
          <thead>
            <tr>
              <th>标签</th>
              <th>摘要</th>
              <th>大小</th>
              <th>创建时间</th>
            </tr>
          </thead>
          <tbody>
            {tags.map((tag) => (
              <tr
                key={tag.name}
                onClick={() => onSelect?.(tag)}
                style={{ cursor: onSelect ? "pointer" : "default" }}
              >
                <td>
                  {tag.name}
                  {stale || tag.stale ? (
                    <span className="badge badge-warning" data-testid="rm-tag-stale-marker">
                      缓存已过期
                    </span>
                  ) : null}
                </td>
                <td>{tag.digest}</td>
                <td>{tag.size}</td>
                <td>{tag.created}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </div>
  );
}
