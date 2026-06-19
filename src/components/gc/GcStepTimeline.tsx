export interface GcStep {
  id: string;
  title: string;
  status: "pending" | "active" | "done" | "error";
  note?: string;
}

export interface GcStepTimelineProps {
  steps: GcStep[];
}

export function GcStepTimeline({ steps }: GcStepTimelineProps) {
  return (
    <div data-testid="rm-gc-step-timeline">
      <div className="gc-section-title">GC 时间线</div>
      <ol className="timeline" role="list" aria-label="垃圾回收步骤">
        {steps.map((step) => (
          <li key={step.id} className={`timeline-step ${step.status}`}>
            <span className="timeline-dot" aria-hidden="true">
              {step.status === "done" ? "✓" : step.status === "error" ? "✕" : ""}
            </span>
            <div className="timeline-content">
              <div className="timeline-title">{step.title}</div>
              {step.note ? <div className="timeline-note">{step.note}</div> : null}
            </div>
          </li>
        ))}
      </ol>
    </div>
  );
}
