# E2E Test Suite — Registry Manager

Playwright E2E specs exercise the browser frontend against the Vite dev server with mocked backend responses.

## STRUCTURE

```
e2e/
├── app.spec.ts                # App shell rendering
├── browse.spec.ts             # Repository, tag, and manifest browsing
├── dashboard.spec.ts          # Empty states and search
├── discovery.spec.ts          # Local registry container discovery
├── local-gc.spec.ts           # Successful local GC workflow
├── local-gc-failure.spec.ts   # GC failure and recovery messaging
├── offline-cache.spec.ts      # Stale cache when registry is offline
├── onboarding.spec.ts         # Docker unavailable and healthy states
├── safe-delete.spec.ts        # Manifest delete confirmation and audit
└── safe-delete-error.spec.ts  # Delete 404 error handling
```

## WHERE TO LOOK

| Concern | Spec |
|---|---|
| App shell visibility | `app.spec.ts` |
| Browse repositories, tags, manifests | `browse.spec.ts` |
| Dashboard empty state and search | `dashboard.spec.ts` |
| Registry container discovery and selection | `discovery.spec.ts` |
| Successful local GC timeline and logs | `local-gc.spec.ts` |
| GC failure recovery instructions | `local-gc-failure.spec.ts` |
| Offline cache banner and stale data | `offline-cache.spec.ts` |
| Docker unavailable and onboarding | `onboarding.spec.ts` |
| Safe manifest delete confirmation | `safe-delete.spec.ts` |
| Delete error states | `safe-delete-error.spec.ts` |

## CONVENTIONS

- Spec files use kebab-case names and the `.spec.ts` suffix.
- Select elements with `page.getByTestId("rm-...")` only.
- Activate mocks by setting `localStorage` keys before navigation or reload.
- Save evidence screenshots to `.sisyphus/evidence/` for task artifacts.
- Tests run against `pnpm dev --host 127.0.0.1` on port `1420`.
- Do not commit `test.only`; CI enables `forbidOnly`.

## ANTI-PATTERNS

- Never use text-only selectors when a stable `rm-` testid exists.
- Never commit `test.only` to the repository.
- Never assume a real registry is available unless the spec explicitly starts one.
- Never run E2E tests without the dev server running on port `1420`.
- Never use hardcoded `page.waitForTimeout` calls; rely on Playwright auto-waiting.
- Never leave mock `localStorage` keys set after a test; clean up in the spec.
