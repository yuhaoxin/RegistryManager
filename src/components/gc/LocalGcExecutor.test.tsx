import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { LocalGcExecutor } from "./LocalGcExecutor";
import { describe, expect, it } from "vitest";

describe("LocalGcExecutor", () => {
  it("renders without placeholder preflight results before GC runs", () => {
    render(<LocalGcExecutor containerId="container-123" containerName="registry" profileId="p1" registryUrl="http://localhost:5000" />);
    expect(screen.getByTestId("rm-gc-preflight-list")).toBeVisible();
    expect(screen.queryByText("Docker daemon")).not.toBeInTheDocument();
    expect(screen.queryByText("Local Docker daemon is reachable")).not.toBeInTheDocument();
  });

  it("renders an empty timeline before GC runs", () => {
    render(<LocalGcExecutor containerId="container-123" containerName="registry" profileId="p1" registryUrl="http://localhost:5000" />);
    expect(screen.getByTestId("rm-gc-step-timeline")).toBeVisible();
    expect(screen.queryByText("Snapshot original state")).not.toBeInTheDocument();
    expect(screen.queryByText("Run garbage-collect")).not.toBeInTheDocument();
  });

  it("renders an empty log panel before GC runs", () => {
    render(<LocalGcExecutor containerId="container-123" containerName="registry" profileId="p1" registryUrl="http://localhost:5000" />);
    expect(screen.getByTestId("rm-gc-live-log-panel")).toBeVisible();
    expect(screen.queryByText(/\[preflight\]/i)).not.toBeInTheDocument();
    expect(screen.queryByText(/\[gc\]/i)).not.toBeInTheDocument();
  });

  it("opens the confirmation dialog when Run GC is clicked", async () => {
    const user = userEvent.setup();
    render(<LocalGcExecutor profileId="p1" registryUrl="http://localhost:5000" />);
    await user.click(screen.getByRole("button", { name: /运行 GC/ }));
    expect(screen.getByTestId("rm-gc-confirm-dialog")).toBeVisible();
  });

  it("does not render without a loopback-local registry URL", () => {
    render(<LocalGcExecutor profileId="p1" />);
    expect(screen.queryByTestId("rm-local-gc-executor")).not.toBeInTheDocument();
    expect(screen.queryByTestId("rm-local-gc-unavailable")).not.toBeInTheDocument();
    expect(screen.queryByRole("button", { name: /运行 GC/ })).not.toBeInTheDocument();
  });
});
