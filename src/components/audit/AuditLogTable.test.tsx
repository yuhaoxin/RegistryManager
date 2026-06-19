import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { AuditEvent } from "../../types";
import { AuditLogTable } from "./AuditLogTable";

const runTauriCommand = vi.fn();

vi.mock("../../hooks/useTauriCommand", () => ({
  runTauriCommand: (...args: unknown[]) => runTauriCommand(...args),
}));

beforeEach(() => {
  runTauriCommand.mockReset();
});

afterEach(() => {
  vi.restoreAllMocks();
});

describe("AuditLogTable", () => {
  it("does not install a polling interval in the component source", () => {
    const source = readFileSync(resolve(process.cwd(), "src/components/audit/AuditLogTable.tsx"), "utf8");

    expect(source).not.toContain("setInterval");
  });

  it("loads audit events once on mount", async () => {
    runTauriCommand.mockResolvedValue([auditEvent("delete_manifest", "pending_gc")]);

    render(<AuditLogTable />);

    await waitFor(() => expect(runTauriCommand).toHaveBeenCalledTimes(1));
    expect(runTauriCommand).toHaveBeenCalledWith("list_audit_events", { limit: 25, offset: 0 });
    expect(screen.getByTestId("rm-audit-log-table")).toHaveTextContent("delete_manifest");
  });

  it("refreshes audit events when the manual button is clicked", async () => {
    const user = userEvent.setup();
    runTauriCommand
      .mockResolvedValueOnce([])
      .mockResolvedValueOnce([auditEvent("local_gc", "gc_completed")]);

    render(<AuditLogTable />);

    await waitFor(() => expect(runTauriCommand).toHaveBeenCalledTimes(1));
    await user.click(screen.getByTestId("rm-refresh-audit-log-button"));

    await waitFor(() => expect(runTauriCommand).toHaveBeenCalledTimes(2));
    expect(runTauriCommand.mock.calls[1]).toEqual(["list_audit_events", { limit: 25, offset: 0 }]);
    expect(screen.getByTestId("rm-audit-log-table")).toHaveTextContent("gc_completed");
  });
});

function auditEvent(action: string, status: string): AuditEvent {
  return {
    id: `${action}-${status}`,
    timestamp: "2026-06-19T10:00:00Z",
    action,
    repositoryName: "alpine",
    digest: "sha256:abc123",
    status,
  };
}
