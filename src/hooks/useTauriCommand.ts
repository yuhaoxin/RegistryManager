import { invoke } from "@tauri-apps/api/core";
import type { AuditEvent, CatalogPage, DeleteImpact, DeleteRepositoryResult, GcResult, ManifestSummary, RegistryHealth, RegistryProfile, TagsPage } from "../types";

type CommandArgs = Record<string, unknown>;

const SELECTED_PROFILE_KEY = "rm-selected-profile";
const CATALOG_CACHE_KEY = "rm-catalog-cache";
const TAG_CACHE_KEY = "rm-tag-cache";
const OFFLINE_KEY = "rm-mock-registry-offline";
const AUDIT_KEY = "rm-audit-events";
const DOCKER_UNAVAILABLE_KEY = "rm-mock-docker-unavailable";
const CATALOG_KEY = "rm-mock-catalog";
const GC_MOCK_KEY = "rm-mock-gc";
const GC_FAILURE_KEY = "rm-mock-gc-failure";
const DELETE_404_KEY = "rm-mock-delete-404";
const DELETE_REPO_PARTIAL_KEY = "rm-mock-delete-repo-partial";
const PROFILES_KEY = "rm-profiles";
const REAL_REGISTRY_KEY = "rm-real-registry-mode";

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
          error: "Docker daemon is not reachable. Start Docker Desktop or the Docker Engine, then refresh Docker status.",
        } as T;
      }
      return { reachable: true, version: undefined, context: "default" } as T;
    case "get_selected_registry_profile":
      return readJson<RegistryProfile | null>(SELECTED_PROFILE_KEY, null) as T;
    case "list_registry_profiles":
      return readJson<RegistryProfile[]>(PROFILES_KEY, []) as T;
    case "create_registry_profile": {
      const input = args.profile as RegistryProfileInput;
      const profile = appendProfile({
        name: input.name ?? "Manual registry",
        registryUrl: input.registryUrl ?? "http://localhost:5000",
        credentialRef: input.credentialRef ?? null,
        containerId: input.containerId ?? null,
        containerName: input.containerName ?? null,
      });
      return profile as T;
    }
    case "update_registry_profile": {
      const profileId = String(args.profileId);
      const input = args.profile as RegistryProfileInput;
      const updated = updateProfileInStore(profileId, {
        name: input.name ?? "Manual registry",
        registryUrl: input.registryUrl ?? "http://localhost:5000",
        credentialRef: input.credentialRef ?? null,
        containerId: input.containerId ?? null,
        containerName: input.containerName ?? null,
      });
      return updated as T;
    }
    case "delete_registry_profile": {
      const profileId = String(args.profileId);
      const profiles = readJson<RegistryProfile[]>(PROFILES_KEY, []).filter((profile) => profile.id !== profileId);
      localStorage.setItem(PROFILES_KEY, JSON.stringify(profiles));
      const selected = readJson<RegistryProfile | null>(SELECTED_PROFILE_KEY, null);
      if (selected?.id === profileId) {
        localStorage.removeItem(SELECTED_PROFILE_KEY);
      }
      return true as T;
    }
    case "select_registry_profile": {
      const input = args.profile as RegistryProfileInput;
      const profile = selectOrCreateProfile({
        name: input.name ?? "Manual registry",
        registryUrl: input.registryUrl ?? "http://localhost:5000",
        credentialRef: input.credentialRef ?? null,
        containerId: input.containerId ?? null,
        containerName: input.containerName ?? null,
      });
      localStorage.setItem(SELECTED_PROFILE_KEY, JSON.stringify(profile));
      return profile as T;
    }
    case "check_registry_health":
      if (isRealRegistryMode()) return checkRealRegistryHealth(profileFromArgs(args)) as T;
      return {
        reachable: !isOffline(),
        status: isOffline() ? "v2_unavailable" : "ok",
        message: isOffline() ? "/v2/ unavailable; using cache." : "/v2/ responded successfully.",
        checkedAt: new Date().toISOString(),
      } as T;
    case "list_catalog":
      if (isRealRegistryMode()) return listRealCatalog(profileFromArgs(args)) as T;
      if (localStorage.getItem(CATALOG_KEY) === "true") return mockCatalogPage() as T;
      return emptyCatalogPage() as T;
    case "list_tags":
      if (isRealRegistryMode()) return listRealTags(profileFromArgs(args), String(args.repository)) as T;
      if (localStorage.getItem(CATALOG_KEY) === "true") return mockTagsPage(String(args.repository)) as T;
      return emptyTagsPage(String(args.repository)) as T;
    case "get_manifest":
      if (isRealRegistryMode()) return getRealManifest(profileFromArgs(args), String(args.repository), String(args.reference)) as T;
      if (localStorage.getItem(CATALOG_KEY) === "true") return mockManifest(String(args.repository), String(args.reference)) as T;
      throw { code: "manifest_not_found", message: "Manifest not found." };
    case "get_delete_impact":
      if (localStorage.getItem(CATALOG_KEY) === "true") return mockDeleteImpact(String(args.repository), String(args.reference)) as T;
      throw { code: "manifest_not_found", message: "Manifest not found." };
    case "delete_manifest":
      if (localStorage.getItem(CATALOG_KEY) === "true") return mockDeleteManifest(args) as T;
      throw { code: "manifest_not_found", message: "Manifest not found." };
    case "delete_repository": {
      const repository = String(args.repository);
      if (isRealRegistryMode()) return deleteRealRepository(profileFromArgs(args), repository) as T;
      if (localStorage.getItem(CATALOG_KEY) !== "true") {
        throw { code: "registry_unreachable", message: "Registry is unreachable. Enable rm-mock-catalog to simulate repository deletion." };
      }
      if (localStorage.getItem(DELETE_REPO_PARTIAL_KEY) === "true") {
        return mockDeleteRepositoryPartial(repository) as T;
      }
      return mockDeleteRepository(repository) as T;
    }
    case "run_local_gc":
      if (localStorage.getItem(GC_MOCK_KEY) === "true" || localStorage.getItem(GC_FAILURE_KEY) === "true") {
        return mockLocalGc() as T;
      }
      throw { code: "gc_not_configured", message: "GC mock is disabled. Set rm-mock-gc in localStorage to enable." };
    case "list_audit_events":
      return readJson<AuditEvent[]>(AUDIT_KEY, []) as T;
    case "refresh_registry":
      return { profileId: args.profileId, refreshedRepositories: 0, cancelled: false, timedOut: false } as T;
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

function emptyCatalogPage(): CatalogPage {
  return { repositories: [], stale: false, lastSyncedAt: new Date().toISOString() };
}

function emptyTagsPage(repository: string): TagsPage {
  return { repository, tags: [], stale: false, lastSyncedAt: new Date().toISOString() };
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
  if (localStorage.getItem(DELETE_404_KEY) === "true") {
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

function mockDeleteRepository(repository: string): DeleteRepositoryResult {
  const digest = "sha256:abc123def4567890";
  const cached = readJson<CatalogPage | null>(CATALOG_CACHE_KEY, null);
  if (!cached) {
    throw { code: "repository_not_found", message: `Repository ${repository} not found in catalog.` };
  }
  const remaining = cached.repositories.filter((entry) => entry.repositoryName !== repository);
  if (remaining.length === cached.repositories.length) {
    throw { code: "repository_not_found", message: `Repository ${repository} not found in catalog.` };
  }
  localStorage.setItem(CATALOG_CACHE_KEY, JSON.stringify({ ...cached, repositories: remaining }));
  return {
    repository,
    status: "success",
    totalTags: 1,
    totalDigests: 1,
    deletedDigests: [digest],
    failedDigests: [],
    tagResults: [{ tag: "latest", digest, status: "pending_gc" }],
    digestResults: [{ digest, tags: ["latest"], status: "pending_gc", pendingGc: true }],
    pendingGc: true,
  };
}

function mockDeleteRepositoryPartial(repository: string): DeleteRepositoryResult {
  const digest = "sha256:abc123def4567890";
  return {
    repository,
    status: "partial_failure",
    totalTags: 2,
    totalDigests: 2,
    deletedDigests: [digest],
    failedDigests: [{ digest: "sha256:failed9876543210", tags: ["edge"], status: "failure", pendingGc: false, error: "Simulated digest delete failure." }],
    tagResults: [
      { tag: "latest", digest, status: "pending_gc" },
      { tag: "edge", digest: "sha256:failed9876543210", status: "failure", error: "Simulated tag resolution failure." },
    ],
    digestResults: [
      { digest, tags: ["latest"], status: "pending_gc", pendingGc: true },
      { digest: "sha256:failed9876543210", tags: ["edge"], status: "failure", pendingGc: false, error: "Simulated digest delete failure." },
    ],
    pendingGc: true,
  };
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

function isRealRegistryMode() {
  return localStorage.getItem(REAL_REGISTRY_KEY) === "true";
}

function profileFromArgs(args: CommandArgs): RegistryProfile {
  const profileId = String(args.profileId ?? "");
  const selected = readJson<RegistryProfile | null>(SELECTED_PROFILE_KEY, null);
  const profiles = readJson<RegistryProfile[]>(PROFILES_KEY, []);
  const profile = profiles.find((item) => item.id === profileId) ?? selected;
  if (!profile) throw { code: "profile_not_selected", message: "Select a registry profile first." };
  assertLocalRegistryUrl(profile.registryUrl);
  return profile;
}

function assertLocalRegistryUrl(registryUrl: string) {
  const url = new URL(registryUrl);
  const host = url.hostname.toLowerCase();
  const isLocal = host === "localhost" || host === "127.0.0.1" || host === "::1";
  if (!isLocal) {
    throw { code: "remote_registry_forbidden", message: "Real browser QA mode only targets localhost registries." };
  }
}

async function checkRealRegistryHealth(profile: RegistryProfile): Promise<RegistryHealth> {
  try {
    const response = await registryFetch(profile, "/v2/", { method: "GET" });
    return {
      reachable: response.ok,
      status: response.ok ? "ok" : "v2_unavailable",
      message: response.ok ? "/v2/ responded successfully." : `/v2/ returned ${response.status}.`,
      checkedAt: new Date().toISOString(),
    };
  } catch (error) {
    return { reachable: false, status: "v2_unavailable", message: errorMessage(error), checkedAt: new Date().toISOString() };
  }
}

async function listRealCatalog(profile: RegistryProfile): Promise<CatalogPage> {
  const response = await registryFetch(profile, "/v2/_catalog", { method: "GET" });
  if (!response.ok) throw { code: "registry_unreachable", message: `Catalog request failed with ${response.status}.` };
  const body = (await response.json()) as { repositories?: string[] };
  const now = new Date().toISOString();
  const repositories = await Promise.all(
    (body.repositories ?? []).map(async (repositoryName) => {
      const tags = await fetchRealTagNames(profile, repositoryName);
      return { registryId: profile.id, repositoryName, tagCount: tags.length, lastSyncedAt: now, syncStatus: "fresh" };
    })
  );
  return { repositories: repositories.filter((repo) => repo.tagCount > 0), stale: false, lastSyncedAt: now };
}

async function listRealTags(profile: RegistryProfile, repository: string): Promise<TagsPage> {
  const tagNames = await fetchRealTagNames(profile, repository);
  const now = new Date().toISOString();
  const tags = await Promise.all(
    tagNames.map(async (tag) => {
      const manifest = await getRealManifest(profile, repository, tag);
      return {
        registryId: profile.id,
        repositoryName: repository,
        tag,
        digest: manifest.digest,
        mediaType: manifest.mediaType,
        rawJson: manifest.rawJson,
        lastSyncedAt: now,
      };
    })
  );
  return { repository, tags, stale: false, lastSyncedAt: now };
}

async function fetchRealTagNames(profile: RegistryProfile, repository: string): Promise<string[]> {
  const response = await registryFetch(profile, `/v2/${encodeRepository(repository)}/tags/list`, { method: "GET" });
  if (response.status === 404) return [];
  if (!response.ok) throw { code: "tags_unreachable", message: `Tags request for ${repository} failed with ${response.status}.` };
  const body = (await response.json()) as { tags?: string[] | null };
  return body.tags ?? [];
}

async function getRealManifest(profile: RegistryProfile, repository: string, reference: string): Promise<ManifestSummary> {
  const response = await registryFetch(profile, `/v2/${encodeRepository(repository)}/manifests/${encodeURIComponent(reference)}`, {
    method: "GET",
    headers: manifestHeaders(),
  });
  if (!response.ok) throw { code: "manifest_not_found", message: `Manifest ${reference} was not found in ${repository}.` };
  const rawJson = await response.text();
  const parsed = JSON.parse(rawJson) as {
    mediaType?: string;
    layers?: Array<{ digest: string; size?: number; mediaType?: string }>;
    manifests?: Array<{ platform?: { os?: string; architecture?: string } }>;
  };
  return {
    digest: response.headers.get("Docker-Content-Digest") ?? reference,
    mediaType: response.headers.get("Content-Type") ?? parsed.mediaType ?? "application/vnd.docker.distribution.manifest.v2+json",
    size: rawJson.length,
    layers: parsed.layers?.map((layer) => ({ digest: layer.digest, size: layer.size ?? 0, mediaType: layer.mediaType ?? "unknown" })),
    platforms: parsed.manifests?.map((manifest) => manifest.platform ?? {}),
    rawJson,
    stale: false,
  };
}

async function deleteRealRepository(profile: RegistryProfile, repository: string): Promise<DeleteRepositoryResult> {
  const page = await listRealTags(profile, repository);
  const tagsByDigest = new Map<string, string[]>();
  page.tags.forEach((tag) => {
    const existing = tagsByDigest.get(tag.digest) ?? [];
    tagsByDigest.set(tag.digest, [...existing, tag.tag]);
  });

  const digestResults: DeleteRepositoryResult["digestResults"] = [];
  const tagResults: DeleteRepositoryResult["tagResults"] = [];
  const deletedDigests: string[] = [];
  const failedDigests: DeleteRepositoryResult["failedDigests"] = [];

  for (const [digest, tags] of tagsByDigest.entries()) {
    const response = await registryFetch(profile, `/v2/${encodeRepository(repository)}/manifests/${encodeURIComponent(digest)}`, {
      method: "DELETE",
    });
    if (response.ok || response.status === 202 || response.status === 404) {
      deletedDigests.push(digest);
      digestResults.push({ digest, tags, status: "pending_gc", pendingGc: true });
      tags.forEach((tag) => tagResults.push({ tag, digest, status: "pending_gc" }));
    } else {
      const result = { digest, tags, status: "failure", pendingGc: false, error: `DELETE returned ${response.status}.` };
      failedDigests.push(result);
      digestResults.push(result);
      tags.forEach((tag) => tagResults.push({ tag, digest, status: "failure", error: result.error }));
    }
  }

  const status = failedDigests.length ? (deletedDigests.length ? "partial_failure" : "failure") : "success";
  return {
    repository,
    status,
    totalTags: page.tags.length,
    totalDigests: tagsByDigest.size,
    deletedDigests,
    failedDigests,
    tagResults,
    digestResults,
    pendingGc: deletedDigests.length > 0,
  };
}

function registryFetch(profile: RegistryProfile, path: string, init: RequestInit) {
  return fetch(`${profile.registryUrl.replace(/\/$/, "")}${path}`, init);
}

function encodeRepository(repository: string) {
  return repository.split("/").map(encodeURIComponent).join("/");
}

function manifestHeaders() {
  return {
    Accept: [
      "application/vnd.docker.distribution.manifest.v2+json",
      "application/vnd.oci.image.manifest.v1+json",
      "application/vnd.docker.distribution.manifest.list.v2+json",
      "application/vnd.oci.image.index.v1+json",
    ].join(", "),
  };
}

function errorMessage(error: unknown) {
  if (typeof error === "object" && error && "message" in error) return String((error as { message: unknown }).message);
  return String(error);
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
type RegistryProfileInput = Pick<RegistryProfile, "name" | "registryUrl"> & {
  credentialRef?: string | null;
  containerId?: string | null;
  containerName?: string | null;
};

function selectOrCreateProfile(input: RegistryProfileInput): RegistryProfile {
  const profiles = readJson<RegistryProfile[]>(PROFILES_KEY, []);
  const existingIndex = profiles.findIndex((profile) => profile.registryUrl === input.registryUrl);
  const now = new Date().toISOString();
  if (existingIndex >= 0) {
    const existing = profiles[existingIndex];
    const updated: RegistryProfile = {
      ...existing,
      name: input.name,
      credentialRef: input.credentialRef,
      containerId: input.containerId ?? existing.containerId,
      containerName: input.containerName ?? existing.containerName,
      updatedAt: now,
    };
    profiles[existingIndex] = updated;
    localStorage.setItem(PROFILES_KEY, JSON.stringify(profiles));
    return updated;
  }
  const created: RegistryProfile = {
    id: `manual-${Date.now()}`,
    name: input.name,
    registryUrl: input.registryUrl,
    credentialRef: input.credentialRef,
    containerId: input.containerId ?? undefined,
    containerName: input.containerName ?? undefined,
    createdAt: now,
    updatedAt: now,
  };
  localStorage.setItem(PROFILES_KEY, JSON.stringify([...profiles, created]));
  return created;
}

function appendProfile(input: RegistryProfileInput): RegistryProfile {
  const profiles = readJson<RegistryProfile[]>(PROFILES_KEY, []);
  const existing = profiles.find((profile) => normalizeRegistryUrl(profile.registryUrl) === normalizeRegistryUrl(input.registryUrl));
  if (existing) {
    return existing;
  }
  const now = new Date().toISOString();
  const created: RegistryProfile = {
    id: `manual-${Date.now()}`,
    name: input.name,
    registryUrl: input.registryUrl,
    credentialRef: input.credentialRef,
    containerId: input.containerId ?? undefined,
    containerName: input.containerName ?? undefined,
    createdAt: now,
    updatedAt: now,
  };
  localStorage.setItem(PROFILES_KEY, JSON.stringify([...profiles, created]));
  return created;
}

function normalizeRegistryUrl(registryUrl: string) {
  return registryUrl.trim().replace(/\/+$/, "");
}

function updateProfileInStore(profileId: string, input: RegistryProfileInput): RegistryProfile {
  const profiles = readJson<RegistryProfile[]>(PROFILES_KEY, []);
  const index = profiles.findIndex((profile) => profile.id === profileId);
  const now = new Date().toISOString();
  if (index >= 0) {
    if (
      input.registryUrl !== profiles[index].registryUrl &&
      profiles.some(
        (profile) => profile.registryUrl === input.registryUrl && profile.id !== profileId,
      )
    ) {
      throw {
        code: "duplicate_registry_url",
        message: "A registry profile with this URL already exists.",
      };
    }
    profiles[index] = {
      ...profiles[index],
      name: input.name,
      registryUrl: input.registryUrl,
      credentialRef: input.credentialRef,
      containerId: input.containerId ?? profiles[index].containerId,
      containerName: input.containerName ?? profiles[index].containerName,
      updatedAt: now,
    };
    localStorage.setItem(PROFILES_KEY, JSON.stringify(profiles));
    return profiles[index];
  }
  const created: RegistryProfile = {
    id: profileId,
    name: input.name,
    registryUrl: input.registryUrl,
    credentialRef: input.credentialRef,
    containerId: input.containerId ?? undefined,
    containerName: input.containerName ?? undefined,
    createdAt: now,
    updatedAt: now,
  };
  localStorage.setItem(PROFILES_KEY, JSON.stringify([...profiles, created]));
  return created;
}
