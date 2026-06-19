import type { DeleteImpact, RepositoryDeleteImpact } from "../../types";

function isRepositoryImpact(impact: DeleteImpact | RepositoryDeleteImpact): impact is RepositoryDeleteImpact {
  return "totalTags" in impact;
}

export function DeleteImpactList({ impact }: { impact: DeleteImpact | RepositoryDeleteImpact }) {
  if (isRepositoryImpact(impact)) {
    return (
      <div className="gc-section" data-testid="delete-impact-list">
        <div className="gc-section-title">Delete impact</div>
        <ul className="preflight-list">
          <li className="preflight-item"><span>Repository</span><span className="font-mono">{impact.repository}</span></li>
          <li className="preflight-item"><span>Total tags</span><span>{impact.totalTags}</span></li>
          <li className="preflight-item"><span>Unique digests</span><span>{impact.uniqueDigests}</span></li>
          <li className="preflight-item"><span>Affected tags</span><span>{impact.affectedTags.length ? impact.affectedTags.join(", ") : "No cached tag mapping"}</span></li>
          <li className="preflight-item warn"><span>Partial-failure warning</span><span>Some manifests may fail to delete. Successful deletions still require server-side GC before storage is reclaimed.</span></li>
        </ul>
      </div>
    );
  }

  return (
    <div className="gc-section" data-testid="delete-impact-list">
      <div className="gc-section-title">Delete impact</div>
      <ul className="preflight-list">
        <li className="preflight-item"><span>Digest</span><span className="font-mono">{impact.digest}</span></li>
        <li className="preflight-item"><span>Media type</span><span>{impact.mediaType}</span></li>
        <li className="preflight-item"><span>Affected tags</span><span>{impact.affectedTags.length ? impact.affectedTags.join(", ") : "No cached tag mapping"}</span></li>
        {impact.isMultiArch ? <li className="preflight-item warn"><span>Multi-arch/index warning</span><span>This digest is an index/list and may affect multiple platform manifests.</span></li> : null}
      </ul>
    </div>
  );
}
