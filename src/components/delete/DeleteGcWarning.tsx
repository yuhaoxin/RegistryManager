export function DeleteGcWarning() {
  return (
    <div className="preflight-item warn" data-testid="delete-gc-warning">
      Storage may not be released until server-side GC completes. Manifest deletion only removes registry references and marks records pending_gc.
    </div>
  );
}
