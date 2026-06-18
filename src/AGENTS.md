# Registry Manager — Frontend Knowledge Base

## OVERVIEW

`src/` holds the React/TypeScript/Vite UI layer that invokes the Rust backend through Tauri IPC.

## STRUCTURE

```
src/
├── components/<domain>/   # UI grouped by workflow, not by component size
│   ├── dashboard/         # Layout, navigation, and overview widgets
│   ├── repository/        # Repository listing and tag browsing
│   ├── manifest/          # Manifest details and deletion workflows
│   ├── delete/            # Bulk delete and confirmation UI
│   ├── gc/                # Garbage collection trigger/status
│   ├── audit/             # Event log viewer
│   └── common/            # Shared controls exported via index.ts
├── context/               # Dependency-injection providers
├── hooks/                 # Domain hooks wrapping Tauri commands
├── types.ts               # Shared frontend/backend contracts
├── setupTests.ts          # Vitest/jsdom global setup
├── main.tsx               # Vite entry point
└── App.tsx                # Root component tree
```

## WHERE TO LOOK

| Task | Location |
|------|----------|
| Add or change a shared type | `src/types.ts` |
| Add a reusable UI primitive | `src/components/common/` and re-export in `common/index.ts` |
| Add a workflow screen | `src/components/<domain>/` matching the feature |
| Wire a new Tauri command into the UI | `src/hooks/useRegistry.ts` and `src/hooks/useTauriCommand.ts` |
| Change test setup | `src/setupTests.ts` |
| Change root layout | `src/components/dashboard/DashboardLayout.tsx` |

## CONVENTIONS

- Component files use PascalCase (e.g., `DashboardLayout.tsx`).
- Hook files use the `use` prefix (e.g., `useRegistry.ts`).
- Tests live next to the code they test: `*.test.tsx`.
- Components are organized by feature folder (`dashboard/`, `repository/`, `gc/`, etc.).
- Use `rm-` prefixed `data-testid` values for test selectors.
- Source imports use relative paths (`../components/...`). The `@` alias is for Vitest only.
- Mock registry responses in the browser through keys in `useTauriCommand.ts`, prefixed with `rm-mock-`.

## ANTI-PATTERNS

- Do not use text-only selectors in tests. Always prefer `data-testid`.
- Do not import with `@/` in source files. Use relative paths.
- Do not suppress `noUnusedLocals` or `noUnusedParameters` warnings.
- Do not remove or skip the `setupTests.ts` import in Vitest config.
- Do not place unrelated feature components in `common/`.
- Do not call Tauri commands directly from components. Route them through `useTauriCommand`.
