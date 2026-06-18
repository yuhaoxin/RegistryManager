import { useContext, useMemo, useState } from "react";
import { RegistryContext } from "../../context/RegistryContext";
import { EmptyState } from "../common";
import { DockerStatusCard } from "./DockerStatusCard";
import { LocalRegistryContainerPicker } from "./LocalRegistryContainerPicker";
import { RecentActivityCard } from "./RecentActivityCard";
import { RegistryContainerCard } from "./RegistryContainerCard";
import { StorageReclaimCard } from "./StorageReclaimCard";
import { ManifestDrawer } from "../manifest/ManifestDrawer";
import { RepositoryBrowser } from "../repository/RepositoryBrowser";
import { TagBrowser } from "../repository/TagBrowser";
import { LocalGcExecutor } from "../gc/LocalGcExecutor";
import { AuditLogTable } from "../audit/AuditLogTable";
import { AuditEvent, DockerStatus, Manifest, RegistryContainer, Repository, Tag } from "../../types";

export interface DashboardLayoutProps {
  dockerStatus?: DockerStatus;
  containers?: RegistryContainer[];
  repositories?: Repository[];
  recentActivity?: AuditEvent[];
  tags?: Tag[];
  manifest?: Manifest;
  initialSelectedId?: string;
}
const defaultContainers: RegistryContainer[] = [
  {
    id: "registry-local",
    name: "registry",
    image: "registry:2",
    status: "running",
    ports: ["5000:5000"],
    createdAt: "2 days ago",
  },
];

const defaultRepositories: Repository[] = [
  { name: "alpine", tagCount: 3, size: "12.4 MB", lastUpdated: "1 hour ago" },
  { name: "nginx", tagCount: 2, size: "67.1 MB", lastUpdated: "3 hours ago" },
  { name: "redis", tagCount: 4, size: "50.2 MB", lastUpdated: "1 day ago" },
  { name: "my-app/backend", tagCount: 7, size: "124 MB", lastUpdated: "2 days ago" },
];

const defaultTags: Tag[] = [
  { name: "latest", digest: "sha256:abc123…", size: "5.6 MB", created: "2026-06-18" },
  { name: "3.18", digest: "sha256:def456…", size: "5.4 MB", created: "2026-06-10" },
];

const defaultManifest: Manifest = {
  digest: "sha256:abc123def4567890abcdef1234567890abcdef1234567890abcdef1234567890",
  mediaType: "application/vnd.docker.distribution.manifest.v2+json",
  size: 528,
  platform: "linux/arm64",
  rawJson: JSON.stringify(
    {
      schemaVersion: 2,
      mediaType: "application/vnd.docker.distribution.manifest.v2+json",
      config: { size: 1472, mediaType: "application/vnd.docker.container.image.v1+json" },
      layers: [{ size: 2813285, mediaType: "application/vnd.docker.image.rootfs.diff.tar.gzip" }],
    },
    null,
    2,
  ),
};

const defaultActivity: AuditEvent[] = [
  {
    id: "evt-1",
    timestamp: "2026-06-18 09:12",
    action: "Manifest deleted",
    repository: "alpine",
    tag: "edge",
    digest: "sha256:abc…",
    status: "success",
  },
];

export function DashboardLayout({
  dockerStatus,
  containers,
  repositories,
  recentActivity = defaultActivity,
  tags,
  manifest,
  initialSelectedId,
}: DashboardLayoutProps) {
  const registry = useContext(RegistryContext);
  const resolvedDockerStatus = dockerStatus ?? registry?.dockerStatus ?? { available: true, version: "29.4.0", context: "default" };
  const resolvedContainers = containers ?? registry?.containers ?? defaultContainers;
  const resolvedRepositories = repositories ?? registry?.repositories ?? defaultRepositories;
  const resolvedTags = tags ?? registry?.tags ?? defaultTags;
  const resolvedManifest = manifest ?? registry?.manifest ?? defaultManifest;

  const [selectedId, setSelectedId] = useState<string | undefined>(initialSelectedId);
  const [search, setSearch] = useState("");
  const [drawerOpen, setDrawerOpen] = useState(false);
  const [selectedRepo, setSelectedRepo] = useState<string | undefined>(undefined);

  const selectedContainer = useMemo(
    () => registry?.selectedContainer ?? resolvedContainers.find((c) => c.id === selectedId),
    [registry?.selectedContainer, resolvedContainers, selectedId],
  );

  const handleRepoClick = (repo: Repository) => {
    setSelectedRepo(repo.name);
    void registry?.selectRepository(repo.name);
    if (!registry) {
      setDrawerOpen(true);
    }
  };

  const handleTagClick = (tag: Tag) => {
    const repository = registry?.selectedRepository ?? selectedRepo ?? resolvedRepositories[0]?.name;
    if (repository) {
      void registry?.openManifest(repository, tag.name);
      setSelectedRepo(repository);
      setDrawerOpen(true);
    }
  };

  const handleSelect = (id: string) => {
    setSelectedId(id);
    void registry?.selectContainer(id);
  };

  const hasSelection = Boolean(selectedContainer);

  return (
    <div className="dashboard-layout">
      <aside className="sidebar" aria-label="Main navigation">
        <div className="sidebar-brand">
          <span className="brand-mark" aria-hidden="true">R</span>
          Registry Manager
        </div>
        <nav>
          <ul className="sidebar-nav">
            <li>
              <a href="#dashboard" className="nav-item active">
                🏠 Dashboard
              </a>
            </li>
            <li>
              <a href="#audit" className="nav-item">
                📜 Audit logs
              </a>
            </li>
            <li>
              <a href="#settings" className="nav-item">
                ⚙️ Settings
              </a>
            </li>
          </ul>
        </nav>

        <LocalRegistryContainerPicker
          containers={resolvedContainers}
          selectedId={registry?.selectedContainer?.id ?? selectedId}
          onSelect={handleSelect}
        />
      </aside>

      <main className="main-content">
        <header className="page-header">
          <h1>Dashboard</h1>
          <p>Manage your local Docker Registry V2 from one place.</p>
        </header>

        <section className="card-grid" aria-label="Status cards">
          <DockerStatusCard status={resolvedDockerStatus} />
          <RegistryContainerCard container={selectedContainer} health={registry?.health} />
          <StorageReclaimCard reclaimableBytes={124_000_000} />
          <RecentActivityCard events={recentActivity} />
        </section>

        {!hasSelection ? (
          <EmptyState
            testId="rm-docker-unavailable-empty"
            icon="🐳"
            title="No registry selected"
            description={
              <>
                Select a local <code>registry:2</code> container from the sidebar, or start one with:{" "}
                <code>docker run -d -p 5000:5000 --name registry registry:2</code>
              </>
            }
          />
        ) : (
          <>
            {registry?.error ? <div className="preflight-item warn" role="status">{registry.error}</div> : null}

            <RepositoryBrowser
              repositories={resolvedRepositories}
              search={search}
              stale={registry?.stale}
              nextCursor={registry?.nextCatalogCursor}
              onSearchChange={setSearch}
              onRepositorySelect={handleRepoClick}
              onLoadMore={() => void registry?.loadCatalog(registry.nextCatalogCursor)}
            />

            <TagBrowser repository={registry?.selectedRepository ?? selectedRepo} tags={resolvedTags} stale={registry?.stale} onSelect={handleTagClick} />

            <LocalGcExecutor containerName={selectedContainer?.name ?? "registry"} profileId={registry?.selectedProfile?.id} />
            <AuditLogTable />
          </>
        )}
      </main>

      <ManifestDrawer
        open={drawerOpen}
        repositoryName={selectedRepo ?? "unknown"}
        manifest={resolvedManifest}
        profileId={registry?.selectedProfile?.id}
        onClose={() => setDrawerOpen(false)}
        onDeleted={() => void registry?.refresh()}
      />
    </div>
  );
}
