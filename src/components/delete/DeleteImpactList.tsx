import type { DeleteImpact } from "../../types";

export function DeleteImpactList({ impact }: { impact: DeleteImpact }) {
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
