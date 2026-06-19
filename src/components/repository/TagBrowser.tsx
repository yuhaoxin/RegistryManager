import { Tag } from "../../types";
import { StaleCacheBanner } from "../common";
import { TagTable } from "./TagTable";

export interface TagBrowserProps {
  repository?: string;
  tags: Tag[];
  stale?: boolean;
  onSelect: (tag: Tag) => void;
}

export function TagBrowser({ repository, tags, stale, onSelect }: TagBrowserProps) {
  return (
    <div data-testid="rm-tag-browser">
      {repository ? <div className="card-subtitle mt-2">{repository} 的标签</div> : null}
      {stale ? <StaleCacheBanner message="Registry 离线。正在显示已过期的缓存标签。" /> : null}
      <TagTable tags={tags} stale={stale} onSelect={onSelect} />
    </div>
  );
}
