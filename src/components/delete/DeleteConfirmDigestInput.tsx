export interface DeleteConfirmDigestInputProps {
  suffix: string;
  value: string;
  onChange: (value: string) => void;
}

export function DeleteConfirmDigestInput({ suffix, value, onChange }: DeleteConfirmDigestInputProps) {
  return (
    <label className="form-field" data-testid="delete-confirm-digest-input">
      <span>输入摘要最后 12 个字符以确认：<strong className="font-mono">{suffix}</strong></span>
      <input value={value} onChange={(event) => onChange(event.target.value)} placeholder={suffix} aria-label="摘要后缀确认" />
    </label>
  );
}
