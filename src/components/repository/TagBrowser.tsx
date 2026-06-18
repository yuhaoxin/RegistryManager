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
      {repository ? <div className="card-subtitle mt-2">Tags for {repository}</div> : null}
      {stale ? <StaleCacheBanner message="Registry is offline. Showing cached tags." /> : null}
      <TagTable tags={tags} onSelect={onSelect} />
    </div>
  );
}
