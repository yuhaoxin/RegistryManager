import { useCallback, useEffect, useMemo, useState } from "react";
import type { CatalogPage, DockerStatus, Manifest, ManifestSummary, PortBinding, RegistryContainer, RegistryContainerSummary, RegistryHealth, RegistryProfile, Repository, Tag, TagsPage } from "../types";
import { runTauriCommand } from "./useTauriCommand";

export function useRegistry() {
  const [dockerStatus, setDockerStatus] = useState<DockerStatus>({ reachable: false });
  const [containers, setContainers] = useState<RegistryContainer[]>([]);
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

  const selectedContainer = useMemo(() => {
    if (!selectedProfile) return undefined;
    return containers.find((container) => selectedProfile.containerId.startsWith(container.id) || container.id.startsWith(selectedProfile.containerId));
  }, [containers, selectedProfile]);

  const loadCatalog = useCallback(async (cursor?: string) => {
    if (!selectedProfile) return;
    setLoading(true);
    try {
      const page = await runTauriCommand<CatalogPage>("list_catalog", { profileId: selectedProfile.id, n: 25, last: cursor });
      const mapped = page.repositories.map((repository) => ({
        name: repository.repositoryName,
        tagCount: repository.tagCount,
        lastUpdated: repository.lastSyncedAt ? new Date(repository.lastSyncedAt).toLocaleString() : undefined,
        stale: page.stale,
      }));
      setRepositories((current) => cursor ? dedupeRepositories([...current, ...mapped]) : mapped);
      setNextCatalogCursor(page.nextLast);
      setStale(page.stale);
      setError(page.error);
    } catch (err) {
      setError(errorMessage(err));
    } finally {
      setLoading(false);
    }
  }, [selectedProfile]);

  const selectRepository = useCallback(async (repository: string) => {
    if (!selectedProfile) return;
    setSelectedRepository(repository);
    setLoading(true);
    try {
      const page = await runTauriCommand<TagsPage>("list_tags", { profileId: selectedProfile.id, repository, n: 25 });
      setTags(page.tags.map((tag) => ({
        name: tag.tag,
        digest: tag.digest,
        size: "—",
        created: tag.lastSyncedAt ? new Date(tag.lastSyncedAt).toLocaleDateString() : "—",
        mediaType: tag.mediaType,
        rawJson: tag.rawJson,
        stale: page.stale,
      })));
      setStale(page.stale);
      setError(page.error);
    } catch (err) {
      setError(errorMessage(err));
    } finally {
      setLoading(false);
    }
  }, [selectedProfile]);

  const openManifest = useCallback(async (repository: string, reference: string) => {
    if (!selectedProfile) return;
    setLoading(true);
    try {
      const summary = await runTauriCommand<ManifestSummary>("get_manifest", { profileId: selectedProfile.id, repository, reference });
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
      setStale(Boolean(summary.stale));
    } catch (err) {
      setError(errorMessage(err));
    } finally {
      setLoading(false);
    }
  }, [selectedProfile]);

  const selectContainer = useCallback(async (containerId: string) => {
    const container = containers.find((item) => item.id === containerId);
    if (!container) return;
    setLoading(true);
    try {
      const profile = await runTauriCommand<RegistryProfile>("select_registry_profile", {
        profile: {
          containerId: container.id,
          containerName: container.name,
          image: container.image,
          registryUrl: container.registryUrl ?? "http://localhost:5000",
          portMapping: container.ports.join(", "),
          storageMounts: JSON.stringify(container.mounts ?? []),
        },
      });
      setSelectedProfile(profile);
      const checked = await runTauriCommand<RegistryHealth>("check_registry_health", { profileId: profile.id });
      setHealth(checked);
      setError(checked.reachable ? undefined : checked.message);
    } catch (err) {
      setError(errorMessage(err));
    } finally {
      setLoading(false);
    }
  }, [containers]);

  const refresh = useCallback(async () => {
    if (!selectedProfile) return;
    await runTauriCommand("refresh_registry", { profileId: selectedProfile.id });
    await loadCatalog();
  }, [loadCatalog, selectedProfile]);

  const cancelRefresh = useCallback(async () => {
    if (!selectedProfile) return;
    await runTauriCommand("cancel_refresh", { profileId: selectedProfile.id });
  }, [selectedProfile]);

  useEffect(() => {
    let active = true;
    async function loadInitial() {
      setLoading(true);
      try {
        const [status, discovered, profile] = await Promise.all([
          runTauriCommand<DockerStatus>("get_docker_status"),
          runTauriCommand<RegistryContainerSummary[]>("discover_registry_containers"),
          runTauriCommand<RegistryProfile | null>("get_selected_registry_profile"),
        ]);
        if (!active) return;
        setDockerStatus({ reachable: status.reachable ?? status.available, version: status.version, context: status.context, error: status.error });
        setContainers(discovered.map(mapContainer));
        if (profile) setSelectedProfile(profile);
      } catch (err) {
        if (active) setError(errorMessage(err));
      } finally {
        if (active) setLoading(false);
      }
    }
    loadInitial();
    return () => {
      active = false;
    };
  }, []);

  useEffect(() => {
    if (!selectedProfile) return;
    void (async () => {
      const checked = await runTauriCommand<RegistryHealth>("check_registry_health", { profileId: selectedProfile.id });
      setHealth(checked);
      await loadCatalog();
    })();
  }, [loadCatalog, selectedProfile]);

  return { dockerStatus, containers, selectedProfile, selectedContainer, health, repositories, tags, manifest, selectedRepository, stale, loading, error, nextCatalogCursor, selectContainer, loadCatalog, selectRepository, openManifest, refresh, cancelRefresh };
}

function mapContainer(container: RegistryContainerSummary): RegistryContainer {
  return {
    id: container.id,
    name: container.name,
    image: container.image,
    status: container.state ?? "exited",
    ports: (container.ports ?? []).map(formatPortBinding),
    createdAt: "discovered",
    registryUrl: container.registryUrl,
    mounts: container.mounts,
    healthStatus: container.healthStatus,
  };
}

function formatPortBinding(port: PortBinding) {
  return port.hostPort ? `${port.hostPort}:${port.containerPort}` : `${port.containerPort}`;
}

function dedupeRepositories(repositories: Repository[]) {
  return Array.from(new Map(repositories.map((repo) => [repo.name, repo])).values());
}

function errorMessage(error: unknown) {
  if (typeof error === "object" && error && "message" in error) return String((error as { message: unknown }).message);
  return String(error);
}
