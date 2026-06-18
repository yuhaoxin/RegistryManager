export interface LoadingStateProps {
  message?: string;
  testId?: string;
}

export function LoadingState({ message = "Loading…", testId }: LoadingStateProps) {
  return (
    <div className="state-center" data-testid={testId} role="status" aria-live="polite">
      <div className="state-icon" aria-hidden="true">⏳</div>
      <div className="state-title">{message}</div>
    </div>
  );
}
