export interface GcLiveLogPanelProps {
  logs: string[];
}

export function GcLiveLogPanel({ logs }: GcLiveLogPanelProps) {
  return (
    <div data-testid="rm-gc-live-log-panel">
      <div className="gc-section-title">实时日志</div>
      <pre
        className="log-panel"
        role="log"
        aria-live="polite"
        aria-atomic="false"
      >
        {logs.length === 0 ? "等待开始…" : logs.join("\n")}
      </pre>
    </div>
  );
}
