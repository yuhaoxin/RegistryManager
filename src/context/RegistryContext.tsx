import { createContext, useContext } from "react";
import type { DockerStatus, Manifest, RegistryContainer, RegistryHealth, RegistryProfile, Repository, Tag } from "../types";

export interface RegistryContextValue {
  dockerStatus: DockerStatus;
  containers: RegistryContainer[];
  selectedProfile?: RegistryProfile;
  selectedContainer?: RegistryContainer;
  health?: RegistryHealth;
  repositories: Repository[];
  tags: Tag[];
  manifest?: Manifest;
  selectedRepository?: string;
  stale: boolean;
  loading: boolean;
  error?: string;
  nextCatalogCursor?: string;
  selectContainer: (containerId: string) => Promise<void>;
  loadCatalog: (cursor?: string) => Promise<void>;
  selectRepository: (repository: string) => Promise<void>;
  openManifest: (repository: string, reference: string) => Promise<void>;
  refresh: () => Promise<void>;
  cancelRefresh: () => Promise<void>;
}

export const RegistryContext = createContext<RegistryContextValue | null>(null);

export function useRegistryContext() {
  const value = useContext(RegistryContext);
  if (!value) {
    throw new Error("useRegistryContext must be used inside RegistryContext.Provider");
  }
  return value;
}
