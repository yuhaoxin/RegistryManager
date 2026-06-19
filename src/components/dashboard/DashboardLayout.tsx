import { useContext, useEffect, useRef, useState } from "react";
import { RegistryContext } from "../../context/RegistryContext";
import { EmptyState } from "../common";
import { DockerStatusCard } from "./DockerStatusCard";
import { RegistryProfileManager } from "./RegistryProfileManager";
import { RecentActivityCard } from "./RecentActivityCard";
import { RegistryHealthCard } from "./RegistryHealthCard";
import { ManifestDrawer } from "../manifest/ManifestDrawer";
import { RepositoryBrowser } from "../repository/RepositoryBrowser";
import { TagBrowser } from "../repository/TagBrowser";
import { LocalGcExecutor } from "../gc/LocalGcExecutor";
import { AuditLogTable, type AuditLogTableHandle } from "../audit/AuditLogTable";
import type { AuditEvent, DockerStatus, Manifest, Repository, Tag } from "../../types";
import { isLocalRegistryUrl } from "../../utils/registryUrl";

export interface DashboardLayoutProps {
  dockerStatus?: DockerStatus;
  repositories?: Repository[];
  recentActivity?: AuditEvent[];
  tags?: Tag[];
  manifest?: Manifest;
}

export function DashboardLayout({
  dockerStatus,
  repositories,
  recentActivity,
  tags,
  manifest,
}: DashboardLayoutProps) {
  const registry = useContext(RegistryContext);
  const resolvedDockerStatus = dockerStatus ?? registry?.dockerStatus ?? { reachable: false };
  const resolvedRepositories = repositories ?? registry?.repositories ?? [];
  const resolvedTags = tags ?? registry?.tags ?? [];
  const resolvedManifest = manifest ?? registry?.manifest;
  const resolvedActivity = recentActivity ?? [];
  const selectedRegistryUrl = registry?.selectedProfile?.registryUrl;
  const canRunLocalGc = selectedRegistryUrl ? isLocalRegistryUrl(selectedRegistryUrl) : false;

  const [search, setSearch] = useState("");
  const [drawerOpen, setDrawerOpen] = useState(false);
  const [selectedRepo, setSelectedRepo] = useState<string | undefined>(undefined);
  const auditLogRef = useRef<AuditLogTableHandle>(null);

  const refreshAuditLog = () => {
    void auditLogRef.current?.refresh();
  };

  useEffect(() => {
    setSelectedRepo(undefined);
    setDrawerOpen(false);
  }, [registry?.selectedProfile?.id]);

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

  const handleProfileSelect = (profile: import("../../types").RegistryProfile) => {
    setSelectedRepo(undefined);
    setDrawerOpen(false);
    void registry?.selectProfile(profile);
  };

  const handleProfileCreate = async (input: { name: string; registryUrl: string; credentialRef?: string | null }) => {
    await registry?.createProfile(input);
  };

  const handleProfileUpdate = async (profileId: string, input: { name: string; registryUrl: string; credentialRef?: string | null }) => {
    await registry?.updateProfile(profileId, input);
  };

  const handleProfileDelete = async (profileId: string) => {
    await registry?.deleteProfile(profileId);
  };

  const hasSelection = Boolean(registry?.selectedProfile);

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

        <RegistryProfileManager
          profiles={registry?.profiles ?? []}
          selectedId={registry?.selectedProfile?.id}
          onSelect={handleProfileSelect}
          onCreate={handleProfileCreate}
          onUpdate={handleProfileUpdate}
          onDelete={handleProfileDelete}
        />
      </aside>

      <main className="main-content">
        <header className="page-header">
          <h1>Dashboard</h1>
          <p>Manage your local Docker Registry V2 from one place.</p>
        </header>

        <section className="card-grid" aria-label="Status cards">
          <DockerStatusCard status={resolvedDockerStatus} />
          <RegistryHealthCard
            profileName={registry?.selectedProfile?.name}
            registryUrl={registry?.selectedProfile?.registryUrl}
            health={registry?.health}
            disabled={registry?.loading}
            onRefresh={registry?.refreshHealth}
          />
          <RecentActivityCard events={resolvedActivity} />
        </section>

        {!hasSelection ? (
          <EmptyState
            testId="rm-docker-unavailable-empty"
            icon="🐳"
            title="No registry selected"
            description="Select a registry profile from the sidebar, or add a new one to get started."
          />
        ) : (
          <>
            {registry?.error ? <div className="preflight-item warn" role="status">{registry.error}</div> : null}

<RepositoryBrowser
repositories={resolvedRepositories}
search={search}
stale={registry?.stale}
              nextCursor={registry?.nextCatalogCursor}
              profileId={registry?.selectedProfile?.id}
              registryUrl={registry?.selectedProfile?.registryUrl}
onSearchChange={setSearch}
              onRepositorySelect={handleRepoClick}
              onRepositoryDelete={async (repository) => {
                await registry?.deleteRepository(repository);
              }}
              onAuditEventRecorded={refreshAuditLog}
onLoadMore={() => void registry?.loadCatalog(registry.nextCatalogCursor)}
/>

            <TagBrowser repository={registry?.selectedRepository ?? selectedRepo} tags={resolvedTags} stale={registry?.stale} onSelect={handleTagClick} />

            {canRunLocalGc ? (
              <LocalGcExecutor
                containerId={registry?.selectedProfile?.containerId}
                containerName={registry?.selectedProfile?.containerName}
                profileId={registry?.selectedProfile?.id}
                registryUrl={selectedRegistryUrl}
                onAuditEventRecorded={refreshAuditLog}
              />
            ) : null}
            <AuditLogTable ref={auditLogRef} />
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
            onAuditEventRecorded={refreshAuditLog}
          />
    </div>
  );
}
