# src-tauri Backend Knowledge Base

## OVERVIEW

Rust async backend for the Tauri v2 shell: IPC commands, Docker/registry clients, SQLite cache, audit log, and keyring credentials.

## STRUCTURE

```
src-tauri/src/
├── commands/          # Tauri IPC handlers (~22 commands), AppState, AppError
├── docker/            # Bollard-based Docker Engine client
├── registry/          # Registry v2 HTTP client (reqwest), auth, manifest ops
├── store/             # SQLite cache (sqlx), migrations, models
├── audit/             # Event logging
├── credentials/       # CredentialStore trait + OS keyring impl
├── lib.rs             # Command registration, state init, plugin setup
└── main.rs            # Entry point (keep windows_subsystem attribute)
```

## WHERE TO LOOK

| Task | Location | Notes |
|------|----------|-------|
| Add/modify IPC command | `src/commands/<domain>.rs` + `lib.rs` | One file per domain; add to `generate_handler!` |
| Change backend DTOs | `src/registry/types.rs`, `src/store/models.rs` | Mirror `src/types.ts` manually |
| Docker client change | `src/docker/` | Rejects remote contexts |
| Registry client change | `src/registry/` | Enforces local-only targets for destructive ops |
| Add DB table/query | `src/store/db.rs`, `src/store/models.rs` | sqlx migrations live in `db.rs` |
| Credential storage | `src/credentials/` | Implement `CredentialStore`; persist via keyring only |
| Audit logging | `src/audit/` | Log delete/GC events with status and errors |
| Global state | `src/commands/mod.rs` | Holds `SqlitePool`, refresh abort handles, GC locks |

## CONVENTIONS

- **Module re-exports:** Use `mod.rs` to expose submodules; keep public surface small.
- **Errors:** Domain enums use `thiserror`; convert everything to `AppError` in `commands/mod.rs` for JSON responses.
- **Serde:** Frontend-facing structs use `serde(rename_all = "camelCase")`.
- **Command files:** One file per domain (`registry.rs`, `docker.rs`, `audit.rs`, etc.).
- **State access:** Handlers take `State<AppState>`; read pool via `&state.pool`.
- **Async:** Commands run with `tauri::async_runtime` or `tokio`; always set explicit `tokio::time::timeout`.
- **SQLite:** Use `sqlx` with migrations defined in `store/db.rs`; use `:memory:` in tests.
- **Naming:** `snake_case` modules/functions, PascalCase types.

## ANTI-PATTERNS

- Never add a command without registering it in `lib.rs` (`generate_handler!`).
- Never allow remote Docker contexts; reject `tcp://`, `http*://`, `ssh://`.
- Never allow remote registry targets for delete/GC; enforce loopback or container bindings.
- Never store credentials in plaintext; route through `CredentialStore` → OS keyring.
- Never run concurrent GC on the same container; use `AppState.gc_locks`.
- Never bypass timeouts on registry API or refresh calls.
- Never let Clippy warnings pass (`-D warnings`).
- Do not remove the `windows_subsystem` attribute from `main.rs`.
