export interface DockerStatus {
  reachable?: boolean;
  available?: boolean;
  version?: string;
  context?: string;
  error?: string;
}

export interface RegistryContainer {
  id: string;
  name: string;
  image: string;
  status: "running" | "paused" | "exited" | "restarting" | string;
  ports: string[];
  createdAt: string;
  registryUrl?: string;
  mounts?: ContainerMount[];
  healthStatus?: string;
}

export interface RegistryContainerSummary {
  id: string;
  name: string;
  image: string;
  registryUrl?: string;
  ports: PortBinding[];
  mounts: ContainerMount[];
  state?: string;
  env: string[];
  restartPolicy?: string;
  healthStatus?: string;
}

export interface PortBinding {
  containerPort: number;
  hostIp?: string;
  hostPort?: number;
  protocol: string;
}

export interface ContainerMount {
  source?: string;
  destination?: string;
  mode?: string;
  mountType?: string;
}

export interface RegistryProfile {
  id: string;
  containerId: string;
  containerName: string;
  image: string;
  registryUrl: string;
  portMapping: string;
  configPath?: string;
  storageMounts: string;
  selectedAt: string;
  lastHealthCheckAt?: string;
}

export interface RegistryHealth {
  reachable: boolean;
  status: string;
  message: string;
  checkedAt: string;
}

export interface AuditEvent {
  id: string;
  timestamp: string;
  action: string;
  registryId?: string;
  containerId?: string;
  repositoryName?: string;
  repository?: string;
  tag?: string;
  digest?: string;
  status: "success" | "failure" | "pending" | "pending_gc" | "gc_completed" | "gc_failed" | string;
  durationMs?: number;
  errorMessage?: string;
  logExcerpt?: string;
}

export interface Repository {
  name: string;
  tagCount: number;
  size?: string;
  lastUpdated?: string;
  stale?: boolean;
}

export interface Manifest {
  digest: string;
  mediaType: string;
  size: number;
  platform?: string;
  layers?: Layer[];
  platforms?: Platform[];
  rawJson: string;
  stale?: boolean;
}

export interface ManifestSummary {
  digest: string;
  mediaType: string;
  size: number;
  layers?: Layer[];
  platforms?: Platform[];
  rawJson: string;
  stale?: boolean;
}

export interface Layer {
  digest: string;
  size: number;
  mediaType: string;
}

export interface Platform {
  os?: string;
  architecture?: string;
}

export interface Tag {
  name: string;
  digest: string;
  size: string;
  created: string;
  mediaType?: string;
  rawJson?: string;
  stale?: boolean;
}

export interface CatalogPage {
  repositories: Array<{
    registryId: string;
    repositoryName: string;
    tagCount: number;
    lastSyncedAt?: string;
    syncStatus: string;
  }>;
  nextLast?: string;
  stale: boolean;
  lastSyncedAt?: string;
  error?: string;
}

export interface TagsPage {
  repository: string;
  tags: Array<{
    registryId: string;
    repositoryName: string;
    tag: string;
    digest: string;
    mediaType: string;
    platformSummary?: string;
    rawJson: string;
    lastSyncedAt: string;
  }>;
  nextLast?: string;
  stale: boolean;
  lastSyncedAt?: string;
  error?: string;
}

export interface DeleteImpact {
  repository: string;
  reference: string;
  digest: string;
  digestSuffix: string;
  mediaType: string;
  affectedTags: string[];
  isMultiArch: boolean;
  warning: string;
}

export interface DeleteResult {
  digest: string;
  status: string;
  pendingGc: boolean;
}

export interface GcStep {
  id: string;
  status: string;
  message: string;
}

export interface GcResult {
  transactionId: string;
  status: string;
  exitCode?: number;
  durationMs: number;
  logs: string[];
  steps: GcStep[];
  originalState: string;
  originalImage: string;
  mountSummary: string;
  configPath: string;
  recoveryAction: string;
  finalHealthStatus: string;
}
