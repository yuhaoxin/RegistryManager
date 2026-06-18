# Registry Manager — Project Knowledge Base

**Generated:** 2026-06-18 16:44 UTC  
**Commit:** 92ff4c2f  
**Branch:** main

## OVERVIEW

Registry Manager is a Tauri v2 desktop application for managing local Docker Registry v2 instances. Frontend is React 19 + TypeScript + Vite; backend is Rust (Tokio async) with SQLite, Bollard (Docker), reqwest (registry HTTP), and OS keyring credential storage.

## STRUCTURE

```
.
├── src/                          # React/TypeScript frontend
│   ├── components/<domain>/      # Feature-grouped UI (dashboard, delete, gc, repository, manifest, audit, common)
│   ├── context/                  # RegistryContext state provider
│   ├── hooks/                    # useRegistry, useTauriCommand
│   ├── types.ts                  # Shared frontend/backend type contracts
│   └── *.test.tsx                # Vitest tests co-located with components
├── src-tauri/                    # Rust backend (Tauri core)
│   ├── src/
│   │   ├── commands/             # Tauri IPC handlers (~22 commands)
│   │   ├── docker/               # Docker Engine client (Bollard)
│   │   ├── registry/             # Registry v2 HTTP client (reqwest)
│   │   ├── store/                # SQLite cache (sqlx)
│   │   ├── audit/                # Event logging
│   │   └── credentials/          # OS keyring abstraction
│   ├── tests/                    # Rust integration tests
│   └── tauri.conf.json           # Window, bundle, build hooks
├── e2e/                          # Playwright E2E specs
├── docker-compose.test.yml       # Local registry fixtures for tests
└── .github/workflows/ci.yml      # Cross-platform CI
```

## WHERE TO LOOK

| Task | Location | Notes |
|------|----------|-------|
| Add/modify Tauri IPC command | `src-tauri/src/commands/` + register in `src-tauri/src/lib.rs` | Every command must be listed in `generate_handler!` |
| Change shared types | `src/types.ts` (frontend) + `src-tauri/src/registry/types.rs` / `store/models.rs` (backend) | Keep DTOs in sync manually |
| Add UI component | `src/components/<domain>/` | Use `rm-` prefixed `data-testid`; export via `common/index.ts` for shared components |
| Docker/registry client change | `src-tauri/src/docker/`, `src-tauri/src/registry/` | Both enforce local-only targets |
| Offline cache logic | `src-tauri/src/store/` | SQLite via sqlx; use `:memory:` for tests |
| Credential storage | `src-tauri/src/credentials/` | `CredentialStore` trait; `SystemKeyring` is the only impl |
| E2E test | `e2e/*.spec.ts` | Chromium only; activate mocks via `localStorage` keys |

## CODE MAP

| Symbol | Type | Location | Role |
|--------|------|----------|------|
| `run` | Function | `src-tauri/src/lib.rs:9` | Tauri app builder; registers commands, state, plugins |
| `AppState` | Struct | `src-tauri/src/commands/mod.rs:21` | Global state: `SqlitePool`, refresh abort handles, GC locks |
| `AppError` | Struct | `src-tauri/src/commands/mod.rs:48` | Central error conversion layer (Docker/Registry/Store/Credential → JSON) |
| `RegistryClient` | Struct | `src-tauri/src/registry/client.rs` | Registry v2 HTTP client with auth, manifest fetch/delete |
| `DockerClient` | Struct | `src-tauri/src/docker/client.rs` | Local Docker Engine client wrapper |
| `useRegistry` | Function | `src/hooks/useRegistry.ts` | Central frontend state hook (~17 operations) |
| `runTauriCommand<T>` | Function | `src/hooks/useTauriCommand.ts` | Tauri invoke gateway with browser mock fallback |
| `RegistryContext` | Context | `src/context/RegistryContext.tsx` | Dependency-injection root for registry state |
| `DashboardLayout` | Component | `src/components/dashboard/DashboardLayout.tsx` | Main app layout / orchestrator |

## CONVENTIONS

- **Package manager:** pnpm. CI pins `10.33.0` and uses `--frozen-lockfile`.
- **No ESLint/Prettier.** Frontend linting = `tsc --noEmit`. Rust formatting = `cargo fmt`; linting = `cargo clippy --all-targets --all-features -- -D warnings`.
- **Path alias:** `@` → `./src` (Vitest config; Vite resolves relative imports in source).
- **TypeScript strict:** `noUnusedLocals`, `noUnusedParameters`, `noFallthroughCasesInSwitch` are enabled. Do not suppress unused warnings.
- **Component naming:** PascalCase files (`DashboardLayout.tsx`), `use`-prefixed hooks, `rm-` prefixed `data-testid`.
- **Rust naming:** `snake_case` modules/functions, PascalCase types. `serde(rename_all = "camelCase")` for frontend-facing structs.
- **Tests:** Vitest `src/**/*.test.{ts,tsx}` with jsdom/globals; Playwright `e2e/*.spec.ts`; Rust `cargo test` with `#[tokio::test]` for async.
- **Mock layer:** `useTauriCommand.ts` contains a browser mock fallback controlled by `localStorage` keys (`rm-mock-*`). This mock lives in production code, not test utilities.

## ANTI-PATTERNS (THIS PROJECT)

- **Do not remove** the `#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]` line in `src-tauri/src/main.rs`; it prevents a console window on Windows release builds.
- **Never allow remote Docker contexts** for destructive workflows. `docker/client.rs` rejects `tcp://`, `http*://`, `ssh://`.
- **Never allow remote registry targets** for delete/GC. `commands/registry.rs::ensure_local_registry_target` enforces loopback or discovered container bindings.
- **Do not store credentials in plaintext.** All credential persistence goes through the `CredentialStore` → OS keyring.
- **Do not run concurrent GC** on the same container. `AppState.gc_locks` serializes GC per container.
- **Do not bypass timeouts.** All registry API and refresh calls use explicit `tokio::time::timeout`.
- **Do not let Clippy warnings pass.** CI treats them as errors (`-D warnings`).
- **Do not use Playwright `.only` in CI.** `forbidOnly` is enabled when `CI=true`.

## UNIQUE STYLES

- **Workspace Cargo.toml with one member:** Root `Cargo.toml` declares `[workspace] members = ["src-tauri"]` to leave room for future crates.
- **Rust lib name suffix:** Crate lib is named `registry_manager_lib` (not `registry_manager`) to avoid Windows bin/lib name collision.
- **Browser-first Tauri mock:** The frontend can run standalone in a browser because `useTauriCommand` auto-detects missing Tauri internals and falls back to `mockInvoke`. This keeps unit tests and E2E runnable without a full Tauri build.
- **Feature-folder UI:** Components grouped by workflow domain (`delete/`, `gc/`, `repository/`) rather than by atomic type (`components/`, `containers/`).

## COMMANDS

```bash
# Install
pnpm install

# Development
pnpm tauri dev          # Full desktop app
pnpm dev                # Frontend-only browser dev server on :1420

# Build
pnpm tauri build        # Platform package (runs pnpm build internally)
pnpm build              # Frontend production build + typecheck

# Lint / typecheck
pnpm lint               # tsc --noEmit
cargo fmt --all --check
cargo clippy --all-targets --all-features -- -D warnings

# Test
cargo test
pnpm vitest run
pnpm playwright test

# Docker fixtures for tests
docker compose -f docker-compose.test.yml up -d registry
docker compose -f docker-compose.test.yml --profile auth up -d registry-auth
```

## NOTES

- Vite dev server is pinned to port `1420` (`strictPort: true`). HMR uses `1421` when `TAURI_DEV_HOST` is set.
- Vite ignores `src-tauri/**` to avoid rebuild loops.
- CI runs on `ubuntu-latest`, `macos-latest`, `windows-latest`; Playwright E2E runs only on Linux because it needs the Docker registry fixture.
- `pnpm-lock.yaml` must stay committed; CI uses `--frozen-lockfile`.
- `onlyBuiltDependencies` is restricted to `esbuild`; CI sets this explicitly before install.
