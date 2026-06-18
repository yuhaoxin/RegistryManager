import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { DashboardLayout } from "./DashboardLayout";
import { describe, expect, it } from "vitest";

const mockContainers = [
  {
    id: "registry-local",
    name: "registry",
    image: "registry:2",
    status: "running" as const,
    ports: ["5000:5000"],
    createdAt: "2 days ago",
  },
];

const mockRepositories = [
  { name: "alpine", tagCount: 3, size: "12.4 MB", lastUpdated: "1 hour ago" },
  { name: "nginx", tagCount: 2, size: "67.1 MB", lastUpdated: "3 hours ago" },
];

describe("Dashboard", () => {
  it("renders all main cards", () => {
    render(
      <DashboardLayout
        containers={mockContainers}
        repositories={mockRepositories}
        recentActivity={[]}
      />,
    );

    expect(screen.getByTestId("rm-docker-status-card")).toBeVisible();
    expect(screen.getByTestId("rm-local-registry-container-picker")).toBeVisible();
    expect(screen.getByTestId("rm-storage-reclaim-card")).toBeVisible();
    expect(screen.getByTestId("rm-recent-activity-card")).toBeVisible();
  });

  it("shows empty state when no registry is selected", () => {
    render(
      <DashboardLayout
        containers={mockContainers}
        repositories={mockRepositories}
        recentActivity={[]}
      />,
    );

    expect(screen.getByTestId("rm-docker-unavailable-empty")).toBeVisible();
    expect(screen.getByText(/No registry selected/i)).toBeInTheDocument();
  });

  it("filters repositories by search input", async () => {
    const user = userEvent.setup();
    render(
      <DashboardLayout
        containers={mockContainers}
        repositories={mockRepositories}
        recentActivity={[]}
        initialSelectedId="registry-local"
      />
    );

    const search = screen.getByTestId("rm-repository-search").querySelector("input")!;
    await user.type(search, "no-such-repo");

    expect(screen.getByTestId("no-search-results")).toBeVisible();
    expect(screen.queryAllByTestId("rm-repository-card")).toHaveLength(0);

    await user.clear(search);
    await user.type(search, "alpine");

    expect(screen.queryByTestId("no-search-results")).not.toBeInTheDocument();
    expect(screen.getAllByTestId("rm-repository-card")).toHaveLength(1);
  });

  it("opens manifest drawer when a repository card is clicked", async () => {
    const user = userEvent.setup();
    render(
      <DashboardLayout
        containers={mockContainers}
        repositories={mockRepositories}
        recentActivity={[]}
        initialSelectedId="registry-local"
      />
    );

    await user.click(screen.getByText("alpine"));

    expect(screen.getByTestId("rm-manifest-drawer")).toBeVisible();
    expect(screen.getByText(/Manifest detail/i)).toBeInTheDocument();
  });

  it("shows registry image as the primary selected-container value", () => {
    render(
      <DashboardLayout
        containers={mockContainers}
        repositories={mockRepositories}
        recentActivity={[]}
        initialSelectedId="registry-local"
      />,
    );

    const card = screen.getByTestId("rm-registry-container-card");
    expect(card).toHaveTextContent("registry:2");
    expect(card).toHaveTextContent("Container: registry");
  });

  it("keeps discovery registry identity constrained inside the picker row", () => {
    render(
      <DashboardLayout
        containers={mockContainers}
        repositories={mockRepositories}
        recentActivity={[]}
      />,
    );

    const picker = screen.getByTestId("rm-local-registry-container-picker");
    expect(picker.querySelector(".registry-picker-row")).toBeInTheDocument();
    expect(picker.querySelector(".registry-picker-identity")).toHaveTextContent("registry");
    expect(picker.querySelector(".registry-picker-image")).toHaveTextContent("registry:2");
    expect(picker.querySelector(".registry-picker-status")).toHaveTextContent("running");
  });

  it("keeps GC timeline pending before GC starts", () => {
    render(
      <DashboardLayout
        containers={mockContainers}
        repositories={mockRepositories}
        recentActivity={[]}
        initialSelectedId="registry-local"
      />,
    );

    expect(screen.getByTestId("rm-gc-step-timeline")).not.toHaveTextContent("✓");
  });
});
