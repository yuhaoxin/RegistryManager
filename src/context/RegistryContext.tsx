import { createContext, useContext } from "react";
import type { DeleteRepositoryResult, DockerStatus, Manifest, RegistryHealth, RegistryProfile, Repository, Tag } from "../types";

export interface RegistryContextValue {
  dockerStatus: DockerStatus;
  profiles: RegistryProfile[];
  selectedProfile?: RegistryProfile;
  health?: RegistryHealth;
  repositories: Repository[];
  tags: Tag[];
  manifest?: Manifest;
  selectedRepository?: string;
  stale: boolean;
  loading: boolean;
  error?: string;
  nextCatalogCursor?: string;
  loadProfiles: () => Promise<void>;
  createProfile: (input: { name: string; registryUrl: string; credentialRef?: string | null }) => Promise<RegistryProfile>;
  updateProfile: (profileId: string, input: { name: string; registryUrl: string; credentialRef?: string | null }) => Promise<RegistryProfile>;
  deleteProfile: (profileId: string) => Promise<void>;
  selectProfile: (profile: RegistryProfile) => Promise<void>;
  refreshHealth: () => Promise<void>;
  loadCatalog: (cursor?: string) => Promise<void>;
  selectRepository: (repository: string) => Promise<void>;
  openManifest: (repository: string, reference: string) => Promise<void>;
  refresh: () => Promise<void>;
  deleteRepository: (repository: string) => Promise<DeleteRepositoryResult | undefined>;
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
