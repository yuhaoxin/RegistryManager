export interface ErrorStateProps {
  title?: string;
  message: React.ReactNode;
  onRetry?: () => void;
  testId?: string;
}

export function ErrorState({ title = "Something went wrong", message, onRetry, testId }: ErrorStateProps) {
  return (
    <div className="state-center" data-testid={testId} role="alert">
      <div className="state-icon" aria-hidden="true">⚠️</div>
      <div className="state-title">{title}</div>
      <div className="state-description">{message}</div>
      {onRetry ? (
        <button type="button" className="btn btn-secondary" onClick={onRetry}>
          Retry
        </button>
      ) : null}
    </div>
  );
}
