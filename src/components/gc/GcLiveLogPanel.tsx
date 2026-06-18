export interface GcLiveLogPanelProps {
  logs: string[];
}

export function GcLiveLogPanel({ logs }: GcLiveLogPanelProps) {
  return (
    <div data-testid="rm-gc-live-log-panel">
      <div className="gc-section-title">Live log</div>
      <pre
        className="log-panel"
        role="log"
        aria-live="polite"
        aria-atomic="false"
      >
        {logs.length === 0 ? "Waiting to start…" : logs.join("\n")}
      </pre>
    </div>
  );
}
