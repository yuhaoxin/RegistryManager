import { RegistryContainer } from "../../types";

export interface LocalRegistryContainerPickerProps {
  containers: RegistryContainer[];
  selectedId?: string;
  onSelect: (id: string) => void;
}

export function LocalRegistryContainerPicker({
  containers,
  selectedId,
  onSelect,
}: LocalRegistryContainerPickerProps) {
  return (
    <div className="card" data-testid="rm-local-registry-container-picker">
      <div className="card-header">
        <div className="card-title">🔍 Local registry discovery</div>
      </div>
      <div className="card-body">
        {containers.length === 0 ? (
          <p className="text-secondary">No local registry containers found.</p>
        ) : (
          <ul className="preflight-list" role="listbox" aria-label="Local registry containers">
            {containers.map((container) => (
              <li
                key={container.id}
                role="option"
                aria-selected={selectedId === container.id}
                className={`preflight-item ${selectedId === container.id ? "ok" : ""}`}
              >
                <input
                  type="radio"
                  name="registry-container"
                  id={`registry-${container.id}`}
                  checked={selectedId === container.id}
                  onChange={() => onSelect(container.id)}
                  className="sr-only"
                />
                <label
                  htmlFor={`registry-${container.id}`}
                  className="registry-picker-row"
                >
                  <span className="registry-picker-meta">
                    <span className="badge badge-info registry-picker-port">{container.ports[0] ?? "no port"}</span>
                    <span className="registry-picker-status">
                      {container.status}
                    </span>
                  </span>
                  <span className="registry-picker-identity">
                    <span className="registry-picker-name" title={container.name}>{container.name}</span>
                    <span className="registry-picker-image" title={container.image}>{container.image}</span>
                  </span>
                </label>
              </li>
            ))}
          </ul>
        )}
      </div>
    </div>
  );
}
