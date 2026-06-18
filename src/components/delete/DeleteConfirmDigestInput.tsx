export interface DeleteConfirmDigestInputProps {
  suffix: string;
  value: string;
  onChange: (value: string) => void;
}

export function DeleteConfirmDigestInput({ suffix, value, onChange }: DeleteConfirmDigestInputProps) {
  return (
    <label className="form-field" data-testid="delete-confirm-digest-input">
      <span>Type the last 12 digest characters to confirm: <strong className="font-mono">{suffix}</strong></span>
      <input value={value} onChange={(event) => onChange(event.target.value)} placeholder={suffix} aria-label="Digest suffix confirmation" />
    </label>
  );
}
