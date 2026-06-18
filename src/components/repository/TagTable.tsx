import { Tag } from "../../types";

export interface TagTableProps {
  tags: Tag[];
  onSelect?: (tag: Tag) => void;
}

export function TagTable({ tags, onSelect }: TagTableProps) {
  return (
    <div className="card" data-testid="rm-tag-table">
      <div className="card-header">
        <div className="card-title">🏷️ Tags</div>
      </div>
      <div className="card-body">
        <table className="data-table">
          <thead>
            <tr>
              <th>Tag</th>
              <th>Digest</th>
              <th>Size</th>
              <th>Created</th>
            </tr>
          </thead>
          <tbody>
            {tags.map((tag) => (
              <tr
                key={tag.name}
                onClick={() => onSelect?.(tag)}
                style={{ cursor: onSelect ? "pointer" : "default" }}
              >
                <td>{tag.name}</td>
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
