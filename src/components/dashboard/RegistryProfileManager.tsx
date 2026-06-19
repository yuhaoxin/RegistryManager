import { useState } from "react";
import type { RegistryProfile } from "../../types";

export interface RegistryProfileManagerProps {
  profiles: RegistryProfile[];
  selectedId?: string;
  onSelect: (profile: RegistryProfile) => void;
  onCreate: (input: { name: string; registryUrl: string; credentialRef?: string | null }) => Promise<void>;
  onUpdate: (profileId: string, input: { name: string; registryUrl: string; credentialRef?: string | null }) => Promise<void>;
  onDelete: (profileId: string) => Promise<void>;
}

interface FormState {
  name: string;
  registryUrl: string;
  credentialRef: string;
}

function emptyForm(): FormState {
  return { name: "", registryUrl: "", credentialRef: "" };
}

export function RegistryProfileManager({
  profiles,
  selectedId,
  onSelect,
  onCreate,
  onUpdate,
  onDelete,
}: RegistryProfileManagerProps) {
  const [isCreating, setIsCreating] = useState(false);
  const [editingId, setEditingId] = useState<string | undefined>(undefined);
  const [form, setForm] = useState<FormState>(emptyForm());
  const [submitting, setSubmitting] = useState(false);
  const [formError, setFormError] = useState<string | undefined>(undefined);
  const [deleteError, setDeleteError] = useState<string | undefined>(undefined);
  const [deletingId, setDeletingId] = useState<string | undefined>(undefined);
  const [pendingDeleteId, setPendingDeleteId] = useState<string | undefined>(undefined);
  const startCreate = () => {
    setEditingId(undefined);
    setForm(emptyForm());
    setFormError(undefined);
    setDeleteError(undefined);
    setPendingDeleteId(undefined);
    setIsCreating(true);
  };

  const startEdit = (profile: RegistryProfile) => {
    setIsCreating(false);
    setEditingId(profile.id);
    setFormError(undefined);
    setDeleteError(undefined);
    setPendingDeleteId(undefined);
    setForm({
      name: profile.name,
      registryUrl: profile.registryUrl,
      credentialRef: profile.credentialRef ?? "",
    });
  };

  const cancelForm = () => {
    setIsCreating(false);
    setEditingId(undefined);
    setForm(emptyForm());
    setFormError(undefined);
    setDeleteError(undefined);
    setPendingDeleteId(undefined);
  };
  const handleSubmit = async (event: { preventDefault: () => void }) => {
    event.preventDefault();
    setSubmitting(true);
    setFormError(undefined);
    try {
      const input = {
        name: form.name.trim(),
        registryUrl: form.registryUrl.trim(),
        credentialRef: form.credentialRef.trim() || null,
      };
      if (editingId) {
        await onUpdate(editingId, input);
      } else {
        await onCreate(input);
      }
      cancelForm();
    } catch (error) {
      setFormError(errorMessage(error));
    } finally {
      setSubmitting(false);
    }
  };

  const handleDelete = async (profileId: string) => {
    setDeleteError(undefined);
    setDeletingId(profileId);
    try {
      await onDelete(profileId);
      setPendingDeleteId(undefined);
    } catch (error) {
      setDeleteError(errorMessage(error));
    } finally {
      setDeletingId(undefined);
    }
  };

  const isFormOpen = isCreating || Boolean(editingId);

  return (
    <div className="card" data-testid="rm-registry-profile-manager">
      <div className="card-header">
        <div className="card-title">Registry 配置</div>
        <button
          type="button"
          className="btn btn-primary btn-sm"
          onClick={startCreate}
          disabled={isFormOpen}
          data-testid="rm-add-profile-button"
        >
          添加
        </button>
      </div>
      <div className="card-body">
        {profiles.length === 0 && !isFormOpen ? (
          <p className="text-secondary" data-testid="rm-no-profiles-message">
            还没有 Registry 配置。添加一个即可开始使用。
          </p>
        ) : null}

        {isFormOpen ? (
          <form className="profile-form" onSubmit={handleSubmit} data-testid="rm-profile-form">
            {formError ? (
              <div className="form-error" role="alert" data-testid="rm-profile-error">
                {formError}
              </div>
            ) : null}
            <div className="form-field">
              <label htmlFor="rm-profile-name">名称</label>
              <input
                id="rm-profile-name"
                type="text"
                className="input"
                value={form.name}
                onChange={(event) => setForm((current) => ({ ...current, name: event.target.value }))}
                placeholder="例如：本地 Registry"
                required
                data-testid="rm-profile-name-input"
              />
            </div>
            <div className="form-field">
              <label htmlFor="rm-profile-url">Registry URL</label>
              <input
                id="rm-profile-url"
                type="url"
                className="input"
                value={form.registryUrl}
                onChange={(event) => setForm((current) => ({ ...current, registryUrl: event.target.value }))}
                placeholder="http://localhost:5000"
                required
                data-testid="rm-profile-url-input"
              />
            </div>
            <div className="form-field">
              <label htmlFor="rm-profile-credential">凭据引用</label>
              <input
                id="rm-profile-credential"
                type="text"
                className="input"
                value={form.credentialRef}
                onChange={(event) => setForm((current) => ({ ...current, credentialRef: event.target.value }))}
                placeholder="可选"
                data-testid="rm-profile-credential-input"
              />
            </div>
            <div className="form-actions">
              <button type="submit" className="btn btn-primary btn-sm" disabled={submitting} data-testid="rm-profile-save-button">
                保存
              </button>
              <button type="button" className="btn btn-secondary btn-sm" onClick={cancelForm} data-testid="rm-profile-cancel-button">
                取消
              </button>
            </div>
          </form>
        ) : null}

        {deleteError ? (
          <div className="form-error" role="alert" data-testid="rm-profile-delete-error">
            {deleteError}
          </div>
        ) : null}

        <ul className="preflight-list" aria-label="Registry 配置" data-testid="rm-profile-list">
          {profiles.map((profile) => (
            <li
              key={profile.id}
              className={`preflight-item registry-profile-item ${selectedId === profile.id ? "ok" : ""}`}
              data-testid="rm-profile-item"
            >
              <input
                type="radio"
                name="registry-profile"
                id={`registry-profile-${profile.id}`}
                checked={selectedId === profile.id}
                onChange={() => onSelect(profile)}
                className="sr-only"
                data-testid="rm-profile-radio"
              />
              <div className="registry-profile-row">
                <label htmlFor={`registry-profile-${profile.id}`} className="registry-picker-row">
                  <span className="registry-picker-meta">
                    <span className="badge badge-info registry-picker-port" title={profile.registryUrl}>
                      {profile.registryUrl}
                    </span>
                  </span>
                  <span className="registry-picker-identity">
                    <span className="registry-picker-name" title={profile.name}>{profile.name}</span>
                    {profile.credentialRef ? (
                      <span className="registry-picker-image">凭据：{profile.credentialRef}</span>
                    ) : null}
                  </span>
                </label>
                <span className="profile-actions">
                  {pendingDeleteId === profile.id ? (
                    <>
                      <button
                        type="button"
                        className="btn btn-secondary btn-sm"
                        onClick={(event) => {
                          event.stopPropagation();
                          setPendingDeleteId(undefined);
                        }}
                        disabled={deletingId === profile.id}
                        data-testid="rm-profile-delete-cancel-button"
                      >
                        取消
                      </button>
                      <button
                        type="button"
                        className="btn btn-danger btn-sm"
                        onClick={(event) => {
                          event.stopPropagation();
                          void handleDelete(profile.id);
                        }}
                        disabled={deletingId === profile.id}
                        data-testid="rm-profile-delete-confirm-button"
                      >
                        {deletingId === profile.id ? "正在删除..." : "确认"}
                      </button>
                    </>
                  ) : (
                    <>
                      <button
                        type="button"
                        className="btn btn-ghost btn-sm"
                        onClick={(event) => {
                          event.stopPropagation();
                          startEdit(profile);
                        }}
                        disabled={isFormOpen}
                        data-testid="rm-profile-edit-button"
                      >
                        编辑
                      </button>
                      <button
                        type="button"
                        className="btn btn-ghost btn-sm"
                        onClick={(event) => {
                          event.stopPropagation();
                          setDeleteError(undefined);
                          setPendingDeleteId(profile.id);
                        }}
                        disabled={isFormOpen}
                        data-testid="rm-profile-delete-button"
                      >
                        删除
                      </button>
                    </>
                  )}
                </span>
              </div>
            </li>
          ))}
        </ul>
      </div>
    </div>
  );
}

function errorMessage(error: unknown) {
  if (typeof error === "object" && error && "message" in error) {
    return String((error as { message: unknown }).message);
  }
  return String(error);
}
