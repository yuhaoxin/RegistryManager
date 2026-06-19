import { useCallback, useEffect, useRef, useState } from "react";
import type { CatalogPage, DeleteRepositoryResult, DockerStatus, Manifest, ManifestSummary, RegistryHealth, RegistryProfile, Repository, Tag, TagsPage } from "../types";
import { runTauriCommand } from "./useTauriCommand";

export interface RegistryProfileInput {
  name: string;
  registryUrl: string;
  credentialRef?: string | null;
  containerId?: string | null;
  containerName?: string | null;
}

type StaleSource = "catalog" | "tags" | "manifest";

interface StaleSources {
  catalog: boolean;
  tags: boolean;
  manifest: boolean;
}

export function useRegistry() {
  const [dockerStatus, setDockerStatus] = useState<DockerStatus>({ reachable: false });
  const [profiles, setProfiles] = useState<RegistryProfile[]>([]);
  const [selectedProfile, setSelectedProfile] = useState<RegistryProfile | undefined>();
  const [health, setHealth] = useState<RegistryHealth | undefined>();
  const [repositories, setRepositories] = useState<Repository[]>([]);
  const [tags, setTags] = useState<Tag[]>([]);
  const [manifest, setManifest] = useState<Manifest | undefined>();
  const [selectedRepository, setSelectedRepository] = useState<string | undefined>();
  const [stale, setStale] = useState(false);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | undefined>();
  const [nextCatalogCursor, setNextCatalogCursor] = useState<string | undefined>();

  const currentProfileIdRef = useRef<string | undefined>(undefined);
  const staleSourcesRef = useRef<StaleSources>(emptyStaleSources());

  const updateStaleSource = useCallback((source: StaleSource, value: boolean) => {
    staleSourcesRef.current = { ...staleSourcesRef.current, [source]: value };
    setStale(staleSourcesRef.current.catalog || staleSourcesRef.current.tags || staleSourcesRef.current.manifest);
  }, []);

  const resetDerivedState = useCallback((profile?: RegistryProfile) => {
    currentProfileIdRef.current = profile?.id;
    staleSourcesRef.current = emptyStaleSources();
    setSelectedProfile(profile);
    setHealth(undefined);
    setRepositories([]);
    setTags([]);
    setManifest(undefined);
    setSelectedRepository(undefined);
    setStale(false);
    setError(undefined);
    setNextCatalogCursor(undefined);
  }, []);
  const loadCatalog = useCallback(async (cursor?: string) => {
    const profileId = currentProfileIdRef.current;
    if (!profileId) return;
    setLoading(true);
    try {
      const page = await runTauriCommand<CatalogPage>("list_catalog", { profileId, n: 25, last: cursor });
      if (currentProfileIdRef.current !== profileId) return;
      const mapped = page.repositories.map((repository) => ({
        name: repository.repositoryName,
        tagCount: repository.tagCount,
        lastUpdated: repository.lastSyncedAt ? new Date(repository.lastSyncedAt).toLocaleString() : undefined,
        stale: page.stale,
      }));
      setRepositories((current) => cursor ? dedupeRepositories([...current, ...mapped]) : mapped);
      setNextCatalogCursor(page.nextLast);
      updateStaleSource("catalog", page.stale);
      setError(page.error);
    } catch (err) {
      if (currentProfileIdRef.current !== profileId) return;
      setError(errorMessage(err));
    } finally {
      if (currentProfileIdRef.current === profileId) setLoading(false);
    }
  }, [updateStaleSource]);

  const selectRepository = useCallback(async (repository: string) => {
    const profileId = currentProfileIdRef.current;
    if (!profileId) return;
    setSelectedRepository(repository);
    setLoading(true);
    try {
      const page = await runTauriCommand<TagsPage>("list_tags", { profileId, repository, n: 25 });
      if (currentProfileIdRef.current !== profileId) return;
      setTags(page.tags.map((tag) => ({
        name: tag.tag,
        digest: tag.digest,
        size: "—",
        created: tag.lastSyncedAt ? new Date(tag.lastSyncedAt).toLocaleDateString() : "—",
        mediaType: tag.mediaType,
        rawJson: tag.rawJson,
        stale: page.stale,
      })));
      updateStaleSource("tags", page.stale);
      setError(page.error);
    } catch (err) {
      if (currentProfileIdRef.current !== profileId) return;
      setError(errorMessage(err));
    } finally {
      if (currentProfileIdRef.current === profileId) setLoading(false);
    }
  }, [updateStaleSource]);

  const openManifest = useCallback(async (repository: string, reference: string) => {
    const profileId = currentProfileIdRef.current;
    if (!profileId) return;
    setLoading(true);
    try {
      const summary = await runTauriCommand<ManifestSummary>("get_manifest", { profileId, repository, reference });
      if (currentProfileIdRef.current !== profileId) return;
      setManifest({
        digest: summary.digest,
        mediaType: summary.mediaType,
        size: summary.size,
        platform: summary.platforms?.[0] ? `${summary.platforms[0].os ?? "unknown"}/${summary.platforms[0].architecture ?? "unknown"}` : undefined,
        layers: summary.layers,
        platforms: summary.platforms,
        rawJson: summary.rawJson,
        stale: summary.stale,
      });
      updateStaleSource("manifest", Boolean(summary.stale));
    } catch (err) {
      if (currentProfileIdRef.current !== profileId) return;
      setError(errorMessage(err));
    } finally {
      if (currentProfileIdRef.current === profileId) setLoading(false);
    }
  }, [updateStaleSource]);

  const loadProfiles = useCallback(async () => {
    const list = await runTauriCommand<RegistryProfile[]>("list_registry_profiles");
    setProfiles(list);
  }, []);

  const createProfile = useCallback(async (input: RegistryProfileInput) => {
    const created = await runTauriCommand<RegistryProfile>("create_registry_profile", {
      profile: {
        name: input.name,
        registryUrl: input.registryUrl,
        credentialRef: input.credentialRef ?? null,
        containerId: input.containerId ?? null,
        containerName: input.containerName ?? null,
      },
    });
    setProfiles((current) => upsertProfile(current, created));
    return created;
  }, []);

  const updateProfile = useCallback(async (profileId: string, input: RegistryProfileInput) => {
    const updated = await runTauriCommand<RegistryProfile>("update_registry_profile", {
      profileId,
      profile: {
        name: input.name,
        registryUrl: input.registryUrl,
        credentialRef: input.credentialRef ?? null,
        containerId: input.containerId ?? null,
        containerName: input.containerName ?? null,
      },
    });
    const nextProfiles = upsertProfile(profiles.map((profile) => (profile.id === profileId ? updated : profile)), updated);
    setProfiles(nextProfiles);
    if (selectedProfile?.id === profileId) {
      setSelectedProfile(nextProfiles.find((profile) => profile.id === updated.id) ?? updated);
    }
    return updated;
  }, [profiles, selectedProfile]);

  const deleteProfile = useCallback(async (profileId: string) => {
    await runTauriCommand("delete_registry_profile", { profileId });
    setProfiles((current) => current.filter((profile) => profile.id !== profileId));
    if (selectedProfile?.id === profileId) {
      resetDerivedState(undefined);
    }
  }, [selectedProfile, resetDerivedState]);

  const selectProfile = useCallback(async (profile: RegistryProfile) => {
    resetDerivedState(profile);
    setLoading(true);
    const startedProfileId = profile.id;
    let resolvedProfileId = profile.id;
    try {
      const selected = await runTauriCommand<RegistryProfile>("select_registry_profile", {
        profile: {
          name: profile.name,
          registryUrl: profile.registryUrl,
          credentialRef: profile.credentialRef,
          containerId: profile.containerId ?? null,
          containerName: profile.containerName ?? null,
        },
      });
      resolvedProfileId = selected.id;
      currentProfileIdRef.current = selected.id;
      setSelectedProfile(selected);
      const checked = await runTauriCommand<RegistryHealth>("check_registry_health", { profileId: selected.id });
      if (currentProfileIdRef.current !== selected.id) return;
      setHealth(checked);
      setError(checked.reachable ? undefined : checked.message);
      const page = await runTauriCommand<CatalogPage>("list_catalog", { profileId: selected.id, n: 25 });
      if (currentProfileIdRef.current !== selected.id) return;
      const mapped = page.repositories.map((repository) => ({
        name: repository.repositoryName,
        tagCount: repository.tagCount,
        lastUpdated: repository.lastSyncedAt ? new Date(repository.lastSyncedAt).toLocaleString() : undefined,
        stale: page.stale,
      }));
      setRepositories(mapped);
      setNextCatalogCursor(page.nextLast);
      updateStaleSource("catalog", page.stale);
      setError(page.error);
    } catch (err) {
      if (currentProfileIdRef.current !== startedProfileId && currentProfileIdRef.current !== resolvedProfileId) return;
      setError(errorMessage(err));
    } finally {
      if (currentProfileIdRef.current === startedProfileId || currentProfileIdRef.current === resolvedProfileId) {
        setLoading(false);
      }
    }
  }, [resetDerivedState, updateStaleSource]);

  const refreshHealth = useCallback(async () => {
    const profileId = currentProfileIdRef.current;
    if (!profileId) return;
    const checked = await runTauriCommand<RegistryHealth>("check_registry_health", { profileId });
    if (currentProfileIdRef.current !== profileId) return;
    setHealth(checked);
    setError(checked.reachable ? undefined : checked.message);
  }, []);

  const refresh = useCallback(async () => {
    const profileId = currentProfileIdRef.current;
    if (!profileId) return;
    await runTauriCommand("refresh_registry", { profileId });
    if (currentProfileIdRef.current !== profileId) return;
    await loadCatalog();
  }, [loadCatalog]);

  const cancelRefresh = useCallback(async () => {
    const profileId = currentProfileIdRef.current;
    if (!profileId) return;
    await runTauriCommand("cancel_refresh", { profileId });
  }, []);

  const deleteRepository = useCallback(async (repository: string) => {
    const profileId = currentProfileIdRef.current;
    if (!profileId) return;
    setLoading(true);
    try {
      const result = await runTauriCommand<DeleteRepositoryResult>("delete_repository", { profileId, repository });
      if (currentProfileIdRef.current !== profileId) return;
      await refresh();
      return result;
    } catch (err) {
      if (currentProfileIdRef.current !== profileId) return;
      setError(errorMessage(err));
      throw err;
    } finally {
      if (currentProfileIdRef.current === profileId) setLoading(false);
    }
  }, [refresh]);


  useEffect(() => {
    let active = true;
    async function loadInitial() {
      setLoading(true);
      try {
        const [status, profile] = await Promise.all([
          runTauriCommand<DockerStatus>("get_docker_status"),
          runTauriCommand<RegistryProfile | null>("get_selected_registry_profile"),
        ]);
        if (!active) return;
        setDockerStatus({ reachable: status.reachable ?? status.available, version: status.version, context: status.context, error: status.error });
        if (profile) {
          setProfiles((current) => upsertProfile(current, profile));
          resetDerivedState(profile);
        }
      } catch (err) {
        if (active) setError(errorMessage(err));
      } finally {
        if (active) setLoading(false);
      }
    }
    void loadProfiles();
    void loadInitial();
    return () => {
      active = false;
    };
  }, [resetDerivedState, loadProfiles]);

  useEffect(() => {
    if (!selectedProfile) return;
    void refreshHealth();
    void loadCatalog();
  }, [loadCatalog, refreshHealth, selectedProfile]);

  return {
    dockerStatus,
    profiles,
    selectedProfile,
    health,
    repositories,
    tags,
    manifest,
    selectedRepository,
    stale,
    loading,
    error,
    nextCatalogCursor,
    loadProfiles,
    createProfile,
    updateProfile,
    deleteProfile,
    selectProfile,
    refreshHealth,
    loadCatalog,
    selectRepository,
    openManifest,
    refresh,
    deleteRepository,
    cancelRefresh,
  };
}

function dedupeRepositories(repositories: Repository[]) {
  return Array.from(new Map(repositories.map((repo) => [repo.name, repo])).values());
}

function upsertProfile(profiles: RegistryProfile[], next: RegistryProfile) {
  const normalizedUrl = normalizeRegistryUrl(next.registryUrl);
  const idIndex = profiles.findIndex((profile) => profile.id === next.id);
  const index = idIndex >= 0 ? idIndex : profiles.findIndex((profile) => normalizeRegistryUrl(profile.registryUrl) === normalizedUrl);
  if (index < 0) return [...profiles, next];
  return profiles.map((profile, currentIndex) => (currentIndex === index ? next : profile));
}

function normalizeRegistryUrl(registryUrl: string) {
  return registryUrl.trim().replace(/\/+$/, "");
}

function emptyStaleSources(): StaleSources {
  return { catalog: false, tags: false, manifest: false };
}

function errorMessage(error: unknown) {
  if (typeof error === "object" && error && "message" in error) return String((error as { message: unknown }).message);
  return String(error);
}
