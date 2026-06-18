import { invoke } from "@tauri-apps/api/core";
import type { AuditEvent, CatalogPage, DeleteImpact, GcResult, RegistryProfile, TagsPage } from "../types";

type CommandArgs = Record<string, unknown>;

const SELECTED_PROFILE_KEY = "rm-selected-profile";
const CATALOG_CACHE_KEY = "rm-catalog-cache";
const TAG_CACHE_KEY = "rm-tag-cache";
const OFFLINE_KEY = "rm-mock-registry-offline";
const AUDIT_KEY = "rm-audit-events";
const DOCKER_UNAVAILABLE_KEY = "rm-mock-docker-unavailable";
const GC_FAILURE_KEY = "rm-mock-gc-failure";

export async function runTauriCommand<T>(command: string, args: CommandArgs = {}): Promise<T> {
  if (hasTauriRuntime()) {
    return invoke<T>(command, args);
  }

  return mockInvoke<T>(command, args);
}

export function useTauriCommand() {
  return { run: runTauriCommand };
}

function hasTauriRuntime() {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}

async function mockInvoke<T>(command: string, args: CommandArgs): Promise<T> {
  switch (command) {
    case "get_docker_status":
      if (localStorage.getItem(DOCKER_UNAVAILABLE_KEY) === "true") {
        return {
          reachable: false,
          version: undefined,
          context: "default",
          error: "Docker daemon is not reachable. Start Docker Desktop or the Docker Engine, then refresh discovery.",
        } as T;
      }
      return { reachable: true, version: "29.4.0", context: "default" } as T;
    case "discover_registry_containers":
      if (localStorage.getItem(DOCKER_UNAVAILABLE_KEY) === "true") return [] as T;
      return [mockContainer()] as T;
    case "get_selected_registry_profile":
      return readJson<RegistryProfile | null>(SELECTED_PROFILE_KEY, null) as T;
    case "select_registry_profile": {
      const input = args.profile as Record<string, string | undefined>;
      const profile: RegistryProfile = {
        id: `local-${input.containerId ?? "registry"}`,
        containerId: input.containerId ?? "registry",
        containerName: input.containerName ?? "registry",
        image: input.image ?? "registry:2",
        registryUrl: input.registryUrl ?? "http://localhost:5000",
        portMapping: input.portMapping ?? "5000:5000",
        configPath: input.configPath,
        storageMounts: input.storageMounts ?? "[]",
        selectedAt: new Date().toISOString(),
      };
      localStorage.setItem(SELECTED_PROFILE_KEY, JSON.stringify(profile));
      return profile as T;
    }
    case "check_registry_health":
      return {
        reachable: !isOffline(),
        status: isOffline() ? "v2_unavailable" : "ok",
        message: isOffline() ? "/v2/ unavailable; using cache." : "/v2/ responded successfully.",
        checkedAt: new Date().toISOString(),
      } as T;
    case "list_catalog":
      return mockCatalogPage() as T;
    case "list_tags":
      return mockTagsPage(String(args.repository)) as T;
    case "get_manifest":
      return mockManifest(String(args.repository), String(args.reference)) as T;
    case "get_delete_impact":
      return mockDeleteImpact(String(args.repository), String(args.reference)) as T;
    case "delete_manifest":
      return mockDeleteManifest(args) as T;
    case "run_local_gc":
      return mockLocalGc() as T;
    case "list_audit_events":
      return readJson<AuditEvent[]>(AUDIT_KEY, []) as T;
    case "refresh_registry":
      return { profileId: args.profileId, refreshedRepositories: 2, cancelled: false, timedOut: false } as T;
    case "cancel_refresh":
      return false as T;
    case "get_cached_repositories":
      return (readJson<CatalogPage | null>(CATALOG_CACHE_KEY, null)?.repositories ?? []) as T;
    case "get_cached_tags":
      return readJson<Record<string, TagsPage>>(TAG_CACHE_KEY, {}) as T;
    default:
      throw new Error(`Mock command not implemented: ${command}`);
  }
}

function mockDeleteImpact(repository: string, reference: string): DeleteImpact {
  const digest = reference.startsWith("sha256:") ? reference : "sha256:abc123def4567890";
  return {
    repository,
    reference,
    digest,
    digestSuffix: digest.slice(-12),
    mediaType: "application/vnd.docker.distribution.manifest.v2+json",
    affectedTags: ["latest"],
    isMultiArch: false,
    warning: "Storage may not be released until server-side GC completes.",
  };
}

function mockDeleteManifest(args: CommandArgs) {
  const digest = String(args.reference ?? "sha256:abc123def4567890");
  if (localStorage.getItem("rm-mock-delete-404") === "true") {
    const event = auditEvent("delete_manifest", "failure", String(args.repository), digest, "Manifest digest was not found in the registry.");
    appendAudit(event);
    throw { code: "manifest_not_found", message: "Manifest digest was not found in the registry." };
  }
  const expected = digest.slice(-12);
  if (String(args.confirmedDigestSuffix) !== expected) {
    throw { code: "delete_confirmation_mismatch", message: "Digest confirmation does not match the required suffix." };
  }
  appendAudit(auditEvent("delete_manifest", "pending_gc", String(args.repository), digest));
  return { digest, status: "pending_gc", pendingGc: true };
}

function mockLocalGc(): GcResult {
  if (localStorage.getItem(GC_FAILURE_KEY) === "true") {
    const result: GcResult = {
      transactionId: `mock-gc-failed-${Date.now()}`,
      status: "gc_failed",
      exitCode: 1,
      durationMs: 310,
      logs: [
        "[snapshot] image=registry:2 state=running mounts=[{\"Type\":\"volume\",\"Destination\":\"/var/lib/registry\"}]",
        "[preflight] invalid config path: /missing/config.yml",
        "registry garbage-collect --delete-untagged /missing/config.yml",
        "configuration error: open /missing/config.yml: no such file or directory",
        "[cleanup] removed temp container",
        "[recovery] original registry container restarted; run docker start registry if it is not healthy",
      ],
      steps: [
        { id: "snapshot", status: "done", message: "Captured original state and exact docker inspect mounts." },
        { id: "stop", status: "done", message: "Stopped original registry before offline GC." },
        { id: "gc", status: "failed", message: "GC failed because the registry config path is invalid." },
        { id: "cleanup", status: "done", message: "Removed temporary GC container." },
        { id: "restart", status: "done", message: "Attempted to restore original running state." },
        { id: "health", status: "failed", message: "Verify registry health manually after fixing the config path." },
      ],
      originalState: "running",
      originalImage: "registry:2",
      mountSummary: "[{\"Type\":\"volume\",\"Destination\":\"/var/lib/registry\"}]",
      configPath: "/missing/config.yml",
      recoveryAction: "Fix REGISTRY_CONFIGURATION_PATH, then run docker start registry and retry GC.",
      finalHealthStatus: "recovery_required",
    };
    appendAudit(auditEvent("local_gc", "gc_failed", undefined, undefined, result.recoveryAction));
    return result;
  }

  const result: GcResult = {
    transactionId: `mock-gc-${Date.now()}`,
    status: "gc_completed",
    exitCode: 0,
    durationMs: 420,
    logs: [
      "[snapshot] image=registry:2 state=running mounts=[{\"Type\":\"volume\",\"Destination\":\"/var/lib/registry\"}]",
      "[stop] original registry container stopped",
      "registry garbage-collect --delete-untagged /etc/docker/registry/config.yml",
      "[cleanup] removed temp container",
      "[health] /v2/ ok",
    ],
    steps: [
      { id: "snapshot", status: "done", message: "Captured original state and exact docker inspect mounts." },
      { id: "stop", status: "done", message: "Stopped original registry before offline GC." },
      { id: "gc", status: "done", message: "Ran temporary registry GC container." },
      { id: "cleanup", status: "done", message: "Removed temporary GC container." },
      { id: "restart", status: "done", message: "Restored original running state." },
      { id: "health", status: "done", message: "/v2/ health check passed." },
    ],
    originalState: "running",
    originalImage: "registry:2",
    mountSummary: "[{\"Type\":\"volume\",\"Destination\":\"/var/lib/registry\"}]",
    configPath: "/etc/docker/registry/config.yml",
    recoveryAction: "restarted_original_container",
    finalHealthStatus: "healthy",
  };
  appendAudit(auditEvent("local_gc", "gc_completed", undefined, undefined));
  return result;
}

function auditEvent(action: string, status: string, repositoryName?: string, digest?: string, errorMessage?: string): AuditEvent {
  return {
    id: `${action}-${Date.now()}-${Math.random().toString(16).slice(2)}`,
    timestamp: new Date().toISOString(),
    action,
    repositoryName,
    digest,
    status,
    errorMessage,
  };
}

function appendAudit(event: AuditEvent) {
  const events = readJson<AuditEvent[]>(AUDIT_KEY, []);
  localStorage.setItem(AUDIT_KEY, JSON.stringify([event, ...events].slice(0, 50)));
}

function mockContainer() {
  return {
    id: "f8912fd523f0",
    name: "registry",
    image: "registry:2",
    registryUrl: "http://localhost:5000",
    ports: [{ containerPort: 5000, hostIp: "0.0.0.0", hostPort: 5000, protocol: "tcp" }],
    mounts: [{ destination: "/var/lib/registry", mountType: "volume" }],
    state: "running",
  };
}

function mockCatalogPage(): CatalogPage {
  const cached = readJson<CatalogPage | null>(CATALOG_CACHE_KEY, null);
  if (isOffline() && cached) {
    return { ...cached, stale: true, error: "Registry is offline." };
  }

  const now = new Date().toISOString();
  const page: CatalogPage = {
    repositories: [
      { registryId: "local-registry", repositoryName: "alpine", tagCount: 1, lastSyncedAt: now, syncStatus: "fresh" },
      { registryId: "local-registry", repositoryName: "busybox", tagCount: 1, lastSyncedAt: now, syncStatus: "fresh" },
    ],
    stale: false,
    lastSyncedAt: now,
  };
  localStorage.setItem(CATALOG_CACHE_KEY, JSON.stringify(page));
  return page;
}

function mockTagsPage(repository: string): TagsPage {
  const cached = readJson<Record<string, TagsPage>>(TAG_CACHE_KEY, {});
  if (isOffline() && cached[repository]) {
    return { ...cached[repository], stale: true, error: "Registry is offline." };
  }

  const now = new Date().toISOString();
  const page: TagsPage = {
    repository,
    tags: [
      {
        registryId: "local-registry",
        repositoryName: repository,
        tag: "latest",
        digest: "sha256:abc123def4567890",
        mediaType: "application/vnd.docker.distribution.manifest.v2+json",
        rawJson: JSON.stringify({ schemaVersion: 2, layers: [{ digest: "sha256:layer1", size: 2813285 }] }, null, 2),
        lastSyncedAt: now,
      },
    ],
    stale: false,
    lastSyncedAt: now,
  };
  localStorage.setItem(TAG_CACHE_KEY, JSON.stringify({ ...cached, [repository]: page }));
  return page;
}

function mockManifest(repository: string, reference: string) {
  return {
    repository,
    reference,
    digest: "sha256:abc123def4567890",
    mediaType: "application/vnd.docker.distribution.manifest.v2+json",
    layers: [{ digest: "sha256:layer1", size: 2813285, mediaType: "application/vnd.docker.image.rootfs.diff.tar.gzip" }],
    platforms: [{ os: "linux", architecture: "arm64" }],
    rawJson: JSON.stringify({ schemaVersion: 2, mediaType: "application/vnd.docker.distribution.manifest.v2+json" }, null, 2),
    size: 512,
    stale: isOffline(),
    lastSyncedAt: new Date().toISOString(),
  };
}

function isOffline() {
  return localStorage.getItem(OFFLINE_KEY) === "true";
}

function readJson<T>(key: string, fallback: T): T {
  const value = localStorage.getItem(key);
  if (!value) return fallback;
  try {
    return JSON.parse(value) as T;
  } catch {
    return fallback;
  }
}
