# Registry Manager

Registry Manager is a Tauri v2 desktop application for managing local Docker Registry v2 instances. It focuses on safe local workflows: manually registering local Registry v2 URLs, browsing repositories and manifests, deleting manifests by digest, executing local storage reclaim, and preserving audit/cache state for offline visibility.

## Features

- Manual local Registry v2 URL profile management.
- Repository, tag, manifest, and digest browsing for Registry v2 endpoints.
- Live registry health/status checks with manual refresh and offline cache fallback.
- Safe manifest deletion with confirmation, impact preview, and audit logging.
- Local Storage Reclaim that executes real local Docker registry garbage collection.
- Offline SQLite cache for previously browsed repositories and manifests.
- System keyring credential storage; no plaintext credential persistence.

## Target platforms

| Platform | Supported baseline | Notes |
| --- | --- | --- |
| macOS | macOS 13+ | Docker Desktop recommended. |
| Windows | Windows 10+ | Docker Desktop with WSL2 backend recommended. |
| Linux | Ubuntu 22.04+ | Docker Engine and Tauri WebKitGTK dependencies required. |

Production signing, notarization, and store packaging are not configured in this repository. Release builds are unsigned unless you provide platform certificates and configure Tauri signing separately.

## Prerequisites

- Rust stable toolchain and Cargo.
- Node.js compatible with the project lockfile.
- pnpm.
- Docker Desktop or Docker Engine.
- Tauri platform prerequisites for your OS.

## Install

```bash
pnpm install
```

## Development

Start the desktop app in Tauri development mode:

```bash
pnpm tauri dev
```

Frontend-only Vite development is available for browser-based UI work:

```bash
pnpm dev
```

## Build

Create a platform package for the current OS:

```bash
pnpm tauri build
```

The Tauri configuration uses product name `Registry Manager`, app identifier `com.yuhaoxin.registry-manager`, a 1280x800 default window, and a macOS `.app` bundle target for the current MVP build. Windows and Linux support are tracked in the platform matrix and CI checks; installer signing/notarization is intentionally not claimed here.

## Test

Run Rust tests:

```bash
cargo test
```

Run frontend unit tests:

```bash
pnpm vitest run
```

Run browser E2E tests:

```bash
pnpm playwright test
```

Run the frontend production build/typecheck:

```bash
pnpm build
```

## Local Docker registry setup

Start a disposable local Registry v2 container on `localhost:5000`:

```bash
docker run -d -p 5000:5000 --name registry registry:2
```

If the name `registry` or port `5000` is already in use, remove or rename your existing test container, or use the repository test fixture on `localhost:5001` where appropriate.

Add the registry in the app by creating a profile with the local endpoint URL, for example `http://localhost:5000`. The dashboard reads live status for the selected profile and refreshes repositories/tags/manifests only when requested or when you manually trigger refresh actions; if live reads fail, previously cached data remains available and is marked as stale.

## Security notes

- Credentials are stored through the operating system keyring, not plaintext project files or SQLite rows.
- Registry actions are intended for local Docker Engine/Desktop contexts. Remote Docker contexts are rejected for destructive workflows.
- Delete and garbage-collection operations are recorded in the audit log with status and error details.
- Do not run destructive workflows against production registries. This MVP is designed for local registry maintenance.

## Deletion and garbage-collection caveats

- Manifest deletion is performed by digest, not by tag. Tags are resolved to immutable manifest digests before deletion.
- Manifest deletion does not guarantee storage reclamation until local garbage collection completes successfully.
- Local Storage Reclaim executes real local Docker registry garbage collection only. It does not trigger or manage garbage collection on remote registries.
- Local GC depends on the registry container configuration, storage mounts, and config path. If those are wrong, the command can fail without reclaiming storage.
- The registry may need to be restarted after GC before clients observe a healthy `/v2/` endpoint.

## Troubleshooting

### Docker daemon unavailable

Start Docker Desktop or Docker Engine before running Docker-backed workflows such as Local Storage Reclaim. On Linux, verify the daemon with `docker ps`.

### Docker permission errors

Ensure your user can access the Docker socket. On Linux, add the user to the `docker` group and start a new login session. Running with `sudo` can work for diagnostics, but it is not recommended for normal desktop app use because it changes environment and credential access.

### Registry URL unavailable

Ensure the selected manual profile points to a local Registry v2 endpoint and that the registry is running, for example:

```bash
docker run -d -p 5000:5000 --name registry registry:2
```

Then verify:

```bash
curl -fsS http://localhost:5000/v2/
```

### GC failure

Check the app audit entry and Docker logs:

```bash
docker logs registry
```

Confirm the registry config path exists inside the container, storage mounts are preserved, and the image supports `registry garbage-collect`. If the app stopped the registry before failure, restart it manually after reviewing logs.

### Restart failure

Manually restart the original registry container:

```bash
docker start <container>
```

Then check health with:

```bash
curl -fsS http://localhost:5000/v2/
```

## Useful commands

```bash
pnpm install
pnpm tauri dev
pnpm tauri build
cargo test
pnpm vitest run
pnpm playwright test
pnpm build
docker run -d -p 5000:5000 --name registry registry:2
```
