export interface GcResultSummaryProps {
  status: "idle" | "running" | "success" | "failure";
  durationMs?: number;
  freedBytes?: number;
  errorMessage?: string;
}

export function GcResultSummary({ status, durationMs, freedBytes, errorMessage }: GcResultSummaryProps) {
  if (status === "idle") {
    return (
      <div className="card" data-testid="rm-gc-result-summary">
        <p className="text-secondary">GC 尚未开始。</p>
      </div>
    );
  }

  if (status === "running") {
    return (
      <div className="card" data-testid="rm-gc-result-summary">
        <div className="flex items-center gap-2">
          <span aria-hidden="true">⏳</span>
          <span>垃圾回收正在进行…</span>
        </div>
      </div>
    );
  }

  const isSuccess = status === "success";

  return (
    <div className="card" data-testid="rm-gc-result-summary">
      <div className="card-header">
        <div className="card-title">{isSuccess ? "✅ GC 已完成" : "❌ GC 失败"}</div>
        <span className={`badge badge-${isSuccess ? "success" : "danger"}`}>{isSuccess ? "成功" : "失败"}</span>
      </div>
      <div className="card-body">
        {typeof durationMs === "number" ? (
          <div className="metric">
            <div className="metric-value">{(durationMs / 1000).toFixed(2)}s</div>
            <div className="metric-label">耗时</div>
          </div>
        ) : null}
        {typeof freedBytes === "number" ? (
          <div className="metric">
            <div className="metric-value">{(freedBytes / 1024 / 1024).toFixed(2)} MB</div>
            <div className="metric-label">预计释放</div>
          </div>
        ) : null}
        {errorMessage ? <p className="text-secondary">{errorMessage}</p> : null}
      </div>
    </div>
  );
}
