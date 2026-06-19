import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { RegistryContext, type RegistryContextValue } from "../../context/RegistryContext";
import { DashboardLayout } from "./DashboardLayout";

const mockRepositories = [
  { name: "alpine", tagCount: 3, size: "12.4 MB", lastUpdated: "1 hour ago" },
  { name: "nginx", tagCount: 2, size: "67.1 MB", lastUpdated: "3 hours ago" },
];

function renderWithContext(ui: React.ReactNode, contextValue?: Partial<RegistryContextValue>) {
  const value: RegistryContextValue = {
    dockerStatus: { reachable: true },
    profiles: [],
    repositories: [],
    tags: [],
    stale: false,
    loading: false,
    loadProfiles: async () => {},
    createProfile: async () => ({ id: "p1", name: "Mock", registryUrl: "http://localhost:5000", createdAt: "", updatedAt: "" }),
    updateProfile: async () => ({ id: "p1", name: "Mock", registryUrl: "http://localhost:5000", createdAt: "", updatedAt: "" }),
    deleteProfile: async () => {},
    selectProfile: async () => {},
    refreshHealth: async () => {},
    loadCatalog: async () => {},
    selectRepository: async () => {},
    openManifest: async () => {},
    refresh: async () => {},
    deleteRepository: async () => undefined,
    cancelRefresh: async () => {},
    ...contextValue,
  };
  return render(<RegistryContext.Provider value={value}>{ui}</RegistryContext.Provider>);
}

describe("Dashboard", () => {
  it("renders all main cards", () => {
    renderWithContext(<DashboardLayout repositories={mockRepositories} recentActivity={[]} />);

    expect(screen.getByTestId("rm-docker-status-card")).toBeVisible();
    expect(screen.getByTestId("rm-registry-health-card")).toBeVisible();
    expect(screen.getByTestId("rm-registry-profile-manager")).toBeVisible();
    expect(screen.getByTestId("rm-recent-activity-card")).toBeVisible();
  });

  it("shows empty state when no registry is selected", () => {
    renderWithContext(<DashboardLayout repositories={mockRepositories} recentActivity={[]} />);

    expect(screen.getByTestId("rm-docker-unavailable-empty")).toBeVisible();
    expect(screen.getByTestId("rm-docker-unavailable-empty")).toHaveTextContent(/未选择 Registry/);
  });

  it("renders a manual registry health refresh button", async () => {
    const user = userEvent.setup();
    const refreshHealth = vi.fn().mockResolvedValue(undefined);
    renderWithContext(<DashboardLayout recentActivity={[]} />, {
      selectedProfile: { id: "p1", name: "Local", registryUrl: "http://localhost:5000", createdAt: "", updatedAt: "" },
      health: { reachable: true, status: "ok", message: "/v2/ 响应成功。", checkedAt: "2026-06-19T10:00:00Z" },
      refreshHealth,
    });

    expect(screen.getByTestId("rm-registry-health-card")).toHaveTextContent("/v2/ 响应成功。");
    await user.click(screen.getByTestId("rm-refresh-health-button"));

    expect(refreshHealth).toHaveBeenCalledTimes(1);
  });

  it("shows local GC for loopback registry profiles without requiring a linked container", () => {
    renderWithContext(<DashboardLayout recentActivity={[]} />, {
      selectedProfile: { id: "p1", name: "Manual", registryUrl: "http://localhost:5000", createdAt: "", updatedAt: "" },
    });

    expect(screen.getByTestId("rm-local-gc-executor")).toBeVisible();
    expect(screen.getByRole("button", { name: /^运行 GC$/ })).toBeVisible();
  });

  it("hides local GC for remote registry profiles", () => {
    renderWithContext(<DashboardLayout recentActivity={[]} />, {
      selectedProfile: { id: "p1", name: "Remote", registryUrl: "https://registry.example.com", createdAt: "", updatedAt: "" },
    });

    expect(screen.queryByTestId("rm-local-gc-executor")).not.toBeInTheDocument();
    expect(screen.queryByRole("button", { name: /^运行 GC$/ })).not.toBeInTheDocument();
  });

  it("filters repositories by search input", async () => {
    const user = userEvent.setup();
    renderWithContext(
      <DashboardLayout repositories={mockRepositories} recentActivity={[]} />,
      { selectedProfile: { id: "p1", name: "Local", registryUrl: "http://localhost:5000", createdAt: "", updatedAt: "" } },
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

  it("opens manifest drawer when a tag is clicked", async () => {
    const user = userEvent.setup();
    renderWithContext(
      <DashboardLayout repositories={mockRepositories} tags={[{ name: "latest", digest: "sha256:abc123", size: "—", created: "—" }]} recentActivity={[]} />,
      { selectedProfile: { id: "p1", name: "Local", registryUrl: "http://localhost:5000", createdAt: "", updatedAt: "" }, selectedRepository: "alpine" },
    );

    await user.click(screen.getByText("latest"));

    expect(screen.getByTestId("rm-manifest-drawer")).toBeVisible();
    expect(screen.getByText(/清单详情/)).toBeInTheDocument();
  });

  it("does not render hardcoded default repositories when no registry is selected", () => {
    renderWithContext(<DashboardLayout repositories={mockRepositories} recentActivity={[]} />);
    expect(screen.queryByText("alpine")).not.toBeInTheDocument();
    expect(screen.queryByText("nginx")).not.toBeInTheDocument();
    expect(screen.queryByText("redis")).not.toBeInTheDocument();
  });

  it("does not render hardcoded default tags or manifests when no registry is selected", () => {
    renderWithContext(<DashboardLayout recentActivity={[]} />);
    expect(screen.queryByText("sha256:abc123…")).not.toBeInTheDocument();
    expect(screen.queryByText("linux/arm64")).not.toBeInTheDocument();
  });

  it("does not render hardcoded default activity when no registry is selected", () => {
    renderWithContext(<DashboardLayout recentActivity={[]} />);
    expect(screen.queryByText("Manifest deleted")).not.toBeInTheDocument();
    expect(screen.getByText("暂无最近活动。")).toBeInTheDocument();
  });

  it("labels stale cached repositories and tags as offline data", () => {
    renderWithContext(<DashboardLayout recentActivity={[]} />, {
      selectedProfile: { id: "p1", name: "Local", registryUrl: "http://localhost:5000", createdAt: "", updatedAt: "" },
      selectedRepository: "alpine",
      stale: true,
      repositories: [{ name: "alpine", tagCount: 1, lastUpdated: "2026-06-19 10:00", stale: true }],
      tags: [{ name: "latest", digest: "sha256:abc123", size: "—", created: "2026-06-19", stale: true }],
    });

    expect(screen.getAllByTestId("stale-cache-banner")).toHaveLength(2);
    expect(screen.getByTestId("rm-repository-stale-marker")).toHaveTextContent(/缓存已过期/);
    expect(screen.getByTestId("rm-tag-stale-marker")).toHaveTextContent(/缓存已过期/);
  });
});
