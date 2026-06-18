export interface EmptyStateProps {
  title: string;
  description: React.ReactNode;
  icon?: React.ReactNode;
  action?: React.ReactNode;
  testId?: string;
}

export function EmptyState({ title, description, icon, action, testId }: EmptyStateProps) {
  return (
    <div className="state-center" data-testid={testId} role="status" aria-live="polite">
      {icon ? <div className="state-icon" aria-hidden="true">{icon}</div> : null}
      <div className="state-title">{title}</div>
      <div className="state-description">{description}</div>
      {action ? <div className="flex gap-2">{action}</div> : null}
    </div>
  );
}
