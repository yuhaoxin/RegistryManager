import type { DeleteImpact, RepositoryDeleteImpact } from "../../types";

function isRepositoryImpact(impact: DeleteImpact | RepositoryDeleteImpact): impact is RepositoryDeleteImpact {
  return "totalTags" in impact;
}

export function DeleteImpactList({ impact }: { impact: DeleteImpact | RepositoryDeleteImpact }) {
  if (isRepositoryImpact(impact)) {
    return (
      <div className="gc-section" data-testid="delete-impact-list">
        <div className="gc-section-title">删除影响</div>
        <ul className="preflight-list">
          <li className="preflight-item"><span>仓库</span><span className="font-mono">{impact.repository}</span></li>
          <li className="preflight-item"><span>标签总数</span><span>{impact.totalTags}</span></li>
          <li className="preflight-item"><span>唯一摘要数</span><span>{impact.uniqueDigests}</span></li>
          <li className="preflight-item"><span>受影响标签</span><span>{impact.affectedTags.length ? impact.affectedTags.join(", ") : "没有缓存的标签映射"}</span></li>
          <li className="preflight-item warn"><span>部分失败警告</span><span>部分清单可能删除失败。成功删除的内容仍需服务端 GC 后才会回收存储空间。</span></li>
        </ul>
      </div>
    );
  }

  return (
    <div className="gc-section" data-testid="delete-impact-list">
      <div className="gc-section-title">删除影响</div>
      <ul className="preflight-list">
        <li className="preflight-item"><span>摘要</span><span className="font-mono">{impact.digest}</span></li>
        <li className="preflight-item"><span>媒体类型</span><span>{impact.mediaType}</span></li>
        <li className="preflight-item"><span>受影响标签</span><span>{impact.affectedTags.length ? impact.affectedTags.join(", ") : "没有缓存的标签映射"}</span></li>
        {impact.isMultiArch ? <li className="preflight-item warn"><span>多架构/索引警告</span><span>此摘要是索引/列表，可能影响多个平台清单。</span></li> : null}
      </ul>
    </div>
  );
}
