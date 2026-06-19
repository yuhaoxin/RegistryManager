# Commands

## OVERVIEW

Tauri IPC surface: every `#[tauri::command]` in this directory is callable from the frontend.

## STRUCTURE

- `mod.rs` — `AppState`, `AppError`, and command module exports.
- `docker.rs` — `get_docker_status`.
- `registry.rs` — profile selection, credentials, health, catalog/tags/manifest reads, refresh: `select_registry_profile`, `set_registry_credentials`, `clear_registry_credentials`, `get_selected_registry_profile`, `check_registry_health`, `list_catalog`, `list_tags`, `get_manifest`, `refresh_registry`, `cancel_refresh`.
- `delete.rs` — `get_delete_impact`, `delete_manifest`.
- `gc.rs` — `run_local_gc`.
- `audit.rs` — `list_audit_events`.
- `cache.rs` — `get_cached_repositories`, `get_cached_tags`.

## WHERE TO LOOK

| Category | File |
|---|---|
| Docker status | `docker.rs` |
| Registry profile & browsing | `registry.rs` |
| Manifest deletion | `delete.rs` |
| Local garbage collection | `gc.rs` |
| Offline cache reads | `cache.rs` |
| Audit log reads | `audit.rs` |
| Shared state/errors | `mod.rs` |

## CONVENTIONS

- One file per domain.
- Mark handlers with `#[tauri::command]`.
- Name functions `snake_case`; prefix is optional.
- Take `State<'_, AppState>` only when DB, refresh tasks, or GC locks are needed.
- Take `Window` only when emitting events to the frontend.
- Return `Result<T, AppError>`; never return raw domain errors.
- Convert errors through `From<DockerError>`, `From<RegistryError>`, `From<StoreError>`, `From<CredentialError>`, `From<uuid::Error>` impls in `mod.rs`.
- `AppState` fields: `pool` (SQLite), `refresh_tasks` (abort handles), `gc_locks` (per-container tokio mutexes).
- Adding a command: implement in the domain file, then add to `generate_handler!` in `src-tauri/src/lib.rs`.

## ANTI-PATTERNS

- Do not register a command in `lib.rs` without an implemented handler.
- Do not return raw `RegistryError`, `DockerError`, or `StoreError` from a command.
- Do not call delete/GC/list APIs without first calling `ensure_local_registry_target`.
- Do not run GC without acquiring the `gc_locks` entry for the target container.
- Do not perform long-running blocking work on the main thread; spawn async tasks.
