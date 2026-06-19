import { act, renderHook, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { useRegistry } from "./useRegistry";

const runTauriCommand = vi.fn();

vi.mock("./useTauriCommand", () => ({
  runTauriCommand: (...args: unknown[]) => runTauriCommand(...args),
}));

beforeEach(() => {
  runTauriCommand.mockReset();
  runTauriCommand.mockImplementation(async (command: string) => {
    switch (command) {
      case "get_docker_status":
        return { reachable: true };
      case "get_selected_registry_profile":
        return null;
      case "list_registry_profiles":
        return [];
      case "select_registry_profile":
        return { id: "p1", name: "Local", registryUrl: "http://localhost:5000", createdAt: "", updatedAt: "" };
      case "check_registry_health":
        return { reachable: true, status: "ok", message: "ok", checkedAt: new Date().toISOString() };
      case "list_catalog":
        return { repositories: [], stale: false, lastSyncedAt: new Date().toISOString() };
      default:
        return undefined;
    }
  });
});

afterEach(() => {
  vi.restoreAllMocks();
});

describe("useRegistry profile operations", () => {
  it("loads profiles on mount", async () => {
    const profiles = [{ id: "p1", name: "Local", registryUrl: "http://localhost:5000", createdAt: "", updatedAt: "" }];
    runTauriCommand.mockImplementation(async (command: string) => {
      if (command === "list_registry_profiles") return profiles;
      return defaultMock(command);
    });

    const { result } = renderHook(() => useRegistry());
    await waitFor(() => expect(result.current.profiles).toEqual(profiles));
  });

  it("keeps every loaded profile visible even when registry URLs match", async () => {
    const profiles = [
      { id: "p1", name: "Local A", registryUrl: "http://localhost:5000", createdAt: "", updatedAt: "" },
      { id: "p2", name: "Local B", registryUrl: "http://localhost:5000/", createdAt: "", updatedAt: "" },
      { id: "p3", name: "Other", registryUrl: "http://localhost:5001", createdAt: "", updatedAt: "" },
    ];
    runTauriCommand.mockImplementation(async (command: string) => {
      if (command === "list_registry_profiles") return profiles;
      return defaultMock(command);
    });

    const { result } = renderHook(() => useRegistry());

    await waitFor(() => expect(result.current.profiles.map((profile) => profile.id)).toEqual(["p1", "p2", "p3"]));
  });

  it("loads startup state from manual profile commands only", async () => {
    const { result } = renderHook(() => useRegistry());

    await waitFor(() => expect(result.current.loading).toBe(false));

    expect(runTauriCommand).toHaveBeenCalledWith("get_docker_status");
    expect(runTauriCommand).toHaveBeenCalledWith("get_selected_registry_profile");
    expect(runTauriCommand).toHaveBeenCalledWith("list_registry_profiles");
  });

  it("keeps the selected startup profile visible when the profile list is empty", async () => {
    const selected = { id: "p1", name: "Local", registryUrl: "http://localhost:5000", createdAt: "", updatedAt: "" };
    runTauriCommand.mockImplementation(async (command: string) => {
      if (command === "list_registry_profiles") return [];
      if (command === "get_selected_registry_profile") return selected;
      return defaultMock(command);
    });

    const { result } = renderHook(() => useRegistry());

    await waitFor(() => expect(result.current.selectedProfile).toEqual(selected));
    await waitFor(() => expect(result.current.profiles).toEqual([selected]));
  });

  it("creates a profile and appends to the list", async () => {
    const { result } = renderHook(() => useRegistry());
    await waitFor(() => expect(result.current.loading).toBe(false));

    const created = { id: "p2", name: "New", registryUrl: "http://localhost:5001", createdAt: "", updatedAt: "" };
    runTauriCommand.mockImplementation(async (command: string, args: Record<string, unknown>) => {
      if (command === "create_registry_profile") return created;
      return defaultMock(command, args);
    });

    await act(async () => {
      await result.current.createProfile({ name: "New", registryUrl: "http://localhost:5001" });
    });

    expect(result.current.profiles).toContainEqual(created);
  });

  it("reuses an existing profile returned by the backend without duplicating it", async () => {
    const existing = { id: "p1", name: "Local", registryUrl: "http://localhost:5000", createdAt: "", updatedAt: "" };
    runTauriCommand.mockImplementation(async (command: string) => {
      if (command === "list_registry_profiles") return [existing];
      if (command === "create_registry_profile") return existing;
      return defaultMock(command);
    });

    const { result } = renderHook(() => useRegistry());
    await waitFor(() => expect(result.current.profiles).toEqual([existing]));

    await act(async () => {
      await result.current.createProfile({ name: "Duplicate", registryUrl: "http://localhost:5000/" });
    });

    expect(result.current.profiles).toEqual([existing]);
  });

  it("keeps the selected profile visible after updating its URL", async () => {
    const local = { id: "p1", name: "Local", registryUrl: "http://localhost:5000", createdAt: "", updatedAt: "" };
    const staging = { id: "p2", name: "Staging", registryUrl: "http://localhost:5001", createdAt: "", updatedAt: "" };
    const updated = { ...staging, registryUrl: "http://localhost:5000/" };
    runTauriCommand.mockImplementation(async (command: string, args: Record<string, unknown>) => {
      if (command === "list_registry_profiles") return [local, staging];
      if (command === "select_registry_profile") return staging;
      if (command === "update_registry_profile") return updated;
      return defaultMock(command, args);
    });

    const { result } = renderHook(() => useRegistry());
    await waitFor(() => expect(result.current.profiles).toEqual([local, staging]));

    await act(async () => {
      await result.current.selectProfile(staging);
    });
    await waitFor(() => expect(result.current.selectedProfile).toEqual(staging));

    await act(async () => {
      await result.current.updateProfile("p2", { name: "Staging", registryUrl: "http://localhost:5000/" });
    });

    expect(result.current.profiles).toEqual([local, updated]);
    expect(result.current.selectedProfile).toEqual(updated);
  });

  it("deletes a profile and removes it from the list", async () => {
    const local = { id: "p1", name: "Local", registryUrl: "http://localhost:5000", createdAt: "", updatedAt: "" };
    const staging = { id: "p2", name: "Staging", registryUrl: "http://localhost:5001", createdAt: "", updatedAt: "" };
    runTauriCommand.mockImplementation(async (command: string, args: Record<string, unknown>) => {
      if (command === "list_registry_profiles") return [local, staging];
      if (command === "delete_registry_profile") return true;
      return defaultMock(command, args);
    });

    const { result } = renderHook(() => useRegistry());
    await waitFor(() => expect(result.current.profiles).toEqual([local, staging]));

    await act(async () => {
      await result.current.deleteProfile("p2");
    });

    expect(runTauriCommand).toHaveBeenCalledWith("delete_registry_profile", { profileId: "p2" });
    expect(result.current.profiles).toEqual([local]);
  });

  it("selecting a profile resets old derived state before fetching", async () => {
    let resolveCatalog: (value: unknown) => void;
    const catalogPromise = new Promise((resolve) => {
      resolveCatalog = resolve;
    });
    runTauriCommand.mockImplementation(async (command: string) => {
      if (command === "list_catalog") return catalogPromise;
      return defaultMock(command);
    });

    const { result } = renderHook(() => useRegistry());
    await waitFor(() => expect(result.current.loading).toBe(false));

    const profile = { id: "p1", name: "Local", registryUrl: "http://localhost:5000", createdAt: "", updatedAt: "" };

    act(() => {
      void result.current.selectProfile(profile);
    });

    await waitFor(() => expect(result.current.selectedProfile).toEqual(profile));
    expect(result.current.repositories).toHaveLength(0);
    expect(result.current.tags).toHaveLength(0);
    expect(result.current.manifest).toBeUndefined();

    act(() => {
      resolveCatalog({
        repositories: [{ registryId: "p1", repositoryName: "alpine", tagCount: 1, syncStatus: "fresh" }],
        stale: false,
        lastSyncedAt: new Date().toISOString(),
      });
    });

    await waitFor(() => expect(result.current.repositories).toHaveLength(1));
  });

  it("ignores late responses from a previously selected profile", async () => {
    let resolveFirstCatalog: (value: unknown) => void;
    const firstCatalogPromise = new Promise((resolve) => {
      resolveFirstCatalog = resolve;
    });

    runTauriCommand.mockImplementation(async (command: string, args: Record<string, unknown>) => {
      if (command === "select_registry_profile") {
        const input = args.profile as { registryUrl: string };
        return { id: input.registryUrl === "http://a" ? "p-a" : "p-b", name: "Profile", registryUrl: input.registryUrl, createdAt: "", updatedAt: "" };
      }
      if (command === "check_registry_health") {
        return { reachable: true, status: "ok", message: "ok", checkedAt: new Date().toISOString() };
      }
      if (command === "list_catalog") {
        const profileId = args.profileId as string;
        if (profileId === "p-a") return firstCatalogPromise;
        return {
          repositories: [{ registryId: profileId, repositoryName: "repo-b", tagCount: 1, syncStatus: "fresh" }],
          stale: false,
          lastSyncedAt: new Date().toISOString(),
        };
      }
      return defaultMock(command, args);
    });

    const { result } = renderHook(() => useRegistry());
    await waitFor(() => expect(result.current.loading).toBe(false));

    const profileA = { id: "p-a", name: "A", registryUrl: "http://a", createdAt: "", updatedAt: "" };
    const profileB = { id: "p-b", name: "B", registryUrl: "http://b", createdAt: "", updatedAt: "" };

    act(() => {
      void result.current.selectProfile(profileA);
    });

    await waitFor(() => expect(result.current.selectedProfile?.id).toBe("p-a"));

    await act(async () => {
      await result.current.selectProfile(profileB);
    });

    act(() => {
      resolveFirstCatalog({
        repositories: [{ registryId: "p-a", repositoryName: "repo-a", tagCount: 1, syncStatus: "fresh" }],
        stale: false,
        lastSyncedAt: new Date().toISOString(),
      });
    });

    await waitFor(() => expect(result.current.repositories).toHaveLength(1));
    expect(result.current.repositories[0].name).toBe("repo-b");
  });

  it("does not clear stale catalog labeling after loading fresh tags", async () => {
    runTauriCommand.mockImplementation(async (command: string, args: Record<string, unknown>) => {
      if (command === "list_catalog") {
        return {
          repositories: [{ registryId: "p1", repositoryName: "alpine", tagCount: 1, lastSyncedAt: "2026-06-19T10:00:00Z", syncStatus: "cached" }],
          stale: true,
          lastSyncedAt: "2026-06-19T10:00:00Z",
          error: "Registry is offline.",
        };
      }
      if (command === "list_tags") {
        return {
          repository: args.repository,
          tags: [{ registryId: "p1", repositoryName: "alpine", tag: "latest", digest: "sha256:abc123", mediaType: "application/json", rawJson: "{}", lastSyncedAt: "2026-06-19T10:01:00Z" }],
          stale: false,
          lastSyncedAt: "2026-06-19T10:01:00Z",
        };
      }
      return defaultMock(command, args);
    });

    const { result } = renderHook(() => useRegistry());
    await waitFor(() => expect(result.current.loading).toBe(false));

    await act(async () => {
      await result.current.selectProfile({ id: "p1", name: "Local", registryUrl: "http://localhost:5000", createdAt: "", updatedAt: "" });
    });
    await waitFor(() => expect(result.current.repositories[0]?.stale).toBe(true));
    expect(result.current.stale).toBe(true);

    await act(async () => {
      await result.current.selectRepository("alpine");
    });

    expect(result.current.tags[0].stale).toBe(false);
    expect(result.current.stale).toBe(true);
  });

  it("deletes a repository and refreshes the catalog", async () => {
    runTauriCommand.mockImplementation(async (command: string, args: Record<string, unknown>) => {
      if (command === "delete_repository") {
        return {
          repository: String(args.repository),
          status: "success",
          totalTags: 1,
          totalDigests: 1,
          deletedDigests: ["sha256:abc"],
          failedDigests: [],
          tagResults: [{ tag: "latest", digest: "sha256:abc", status: "pending_gc" }],
          digestResults: [{ digest: "sha256:abc", tags: ["latest"], status: "pending_gc", pendingGc: true }],
          pendingGc: true,
        };
      }
      if (command === "refresh_registry") return { profileId: args.profileId, refreshedRepositories: 0, cancelled: false, timedOut: false };
      if (command === "list_catalog") {
        return { repositories: [], stale: false, lastSyncedAt: new Date().toISOString() };
      }
      return defaultMock(command, args);
    });

    const { result } = renderHook(() => useRegistry());
    await waitFor(() => expect(result.current.loading).toBe(false));

    await act(async () => {
      await result.current.selectProfile({ id: "p1", name: "Local", registryUrl: "http://localhost:5000", createdAt: "", updatedAt: "" });
    });

    await act(async () => {
      await result.current.deleteRepository("alpine");
    });

    expect(runTauriCommand).toHaveBeenCalledWith("delete_repository", { profileId: "p1", repository: "alpine" });
    await waitFor(() => expect(result.current.repositories).toHaveLength(0));
  });

function defaultMock(command: string, _args?: Record<string, unknown>) {
  switch (command) {
    case "get_docker_status":
      return { reachable: true };
    case "get_selected_registry_profile":
      return null;
    case "list_registry_profiles":
      return [];
    case "select_registry_profile":
      return { id: "p1", name: "Local", registryUrl: "http://localhost:5000", createdAt: "", updatedAt: "" };
    case "check_registry_health":
      return { reachable: true, status: "ok", message: "ok", checkedAt: new Date().toISOString() };
    case "list_catalog":
      return { repositories: [], stale: false, lastSyncedAt: new Date().toISOString() };
    default:
      return undefined;
  }
}
});
