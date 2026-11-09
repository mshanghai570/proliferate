import type {
  DesktopAiProviderModel,
  DesktopAiProvidersBridge,
} from "@proliferate/product-client/host/desktop-bridge";
import type { ProductStorage } from "@proliferate/product-client/host/product-host";
import {
  readPersistedJsonValue,
  writePersistedJson,
} from "#product/lib/infra/persistence/product-storage";

export const NATIVE_PROVIDER_PROFILES_STORAGE_KEY = "native-ai-provider-profiles-v1";

export interface NativeProviderProfile {
  id: string;
  name: string;
  endpoint: string;
  models: DesktopAiProviderModel[];
  refreshedAt: string | null;
  hasKey: boolean;
  discoveryError: string | null;
}

export interface NativeProviderProfileInput {
  name: string;
  endpoint: string;
}

export interface NativeProviderProfileChanges {
  name: string;
  endpoint: string;
}

interface StorageContext {
  storage: ProductStorage;
  captureException: (error: unknown, context?: { tags?: Record<string, string> }) => void;
}

const PROFILE_ID = /^[0-9a-f]{8}-[0-9a-f]{4}-[1-8][0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/;

function isModel(value: unknown): value is DesktopAiProviderModel {
  if (value === null || typeof value !== "object") return false;
  const model = value as Record<string, unknown>;
  return typeof model.id === "string" && model.id.length > 0
    && (model.displayName === undefined || typeof model.displayName === "string");
}

function isProfile(value: unknown): value is NativeProviderProfile {
  if (value === null || typeof value !== "object") return false;
  const profile = value as Record<string, unknown>;
  return typeof profile.id === "string" && PROFILE_ID.test(profile.id)
    && typeof profile.name === "string" && profile.name.trim().length > 0
    && typeof profile.endpoint === "string"
    && Array.isArray(profile.models) && profile.models.every(isModel)
    && (profile.refreshedAt === null || typeof profile.refreshedAt === "string")
    && typeof profile.hasKey === "boolean"
    && (profile.discoveryError === null || typeof profile.discoveryError === "string");
}

function normalizeEndpoint(value: string): string {
  const input = value.trim();
  let endpoint: URL;
  try {
    endpoint = new URL(input);
  } catch {
    throw new Error("Enter a valid provider endpoint URL.");
  }
  if (!(["http:", "https:"].includes(endpoint.protocol))
    || !endpoint.hostname
    || endpoint.username
    || endpoint.password
    || endpoint.hash
    || input.split("://")[1]?.split(/[/?#]/, 1)[0]?.includes("@")) {
    throw new Error("Provider endpoints must be HTTP(S), contain no credentials, and have no fragment.");
  }
  return input.replace(/\/+$/, "");
}

async function saveProfiles(context: StorageContext, profiles: NativeProviderProfile[]): Promise<void> {
  await writePersistedJson(context, NATIVE_PROVIDER_PROFILES_STORAGE_KEY, profiles, {
    tags: { domain: "native_ai_provider_profiles" },
  });
}

export async function loadNativeProviderProfiles(context: StorageContext): Promise<NativeProviderProfile[]> {
  const value = await readPersistedJsonValue<unknown>(context, NATIVE_PROVIDER_PROFILES_STORAGE_KEY, {
    tags: { domain: "native_ai_provider_profiles" },
  });
  return Array.isArray(value) ? value.filter(isProfile) : [];
}

export async function addNativeProviderProfile(
  context: StorageContext,
  current: NativeProviderProfile[],
  input: NativeProviderProfileInput,
): Promise<NativeProviderProfile[]> {
  const name = input.name.trim();
  if (!name) throw new Error("Enter a provider name.");
  if (name.length > 100) throw new Error("Provider names must be 100 characters or fewer.");
  const endpoint = normalizeEndpoint(input.endpoint);
  if (current.some((profile) => profile.endpoint === endpoint)) {
    throw new Error("A provider profile already uses this endpoint.");
  }
  const profile: NativeProviderProfile = {
    id: crypto.randomUUID(),
    name,
    endpoint,
    models: [],
    refreshedAt: null,
    hasKey: false,
    discoveryError: null,
  };
  const profiles = [...current, profile];
  await saveProfiles(context, profiles);
  return profiles;
}

export async function updateNativeProviderProfile(
  context: StorageContext,
  current: NativeProviderProfile[],
  profileId: string,
  changes: NativeProviderProfileChanges,
): Promise<NativeProviderProfile[]> {
  const selected = current.find((profile) => profile.id === profileId);
  if (!selected) throw new Error("Provider profile no longer exists.");
  const name = changes.name.trim();
  if (!name) throw new Error("Enter a provider name.");
  if (name.length > 100) throw new Error("Provider names must be 100 characters or fewer.");
  const endpoint = normalizeEndpoint(changes.endpoint);
  if (current.some((profile) => profile.id !== profileId && profile.endpoint === endpoint)) {
    throw new Error("A provider profile already uses this endpoint.");
  }
  const endpointChanged = endpoint !== selected.endpoint;
  const updated: NativeProviderProfile = {
    ...selected,
    name,
    endpoint,
    ...(endpointChanged ? { models: [], refreshedAt: null, discoveryError: null } : {}),
  };
  const profiles = current.map((profile) => profile.id === profileId ? updated : profile);
  await saveProfiles(context, profiles);
  return profiles;
}

export async function discoverNativeProviderModels(
  context: StorageContext,
  current: NativeProviderProfile[],
  profileId: string,
  native: DesktopAiProvidersBridge,
): Promise<NativeProviderProfile[]> {
  const selected = current.find((profile) => profile.id === profileId);
  if (!selected) throw new Error("Provider profile no longer exists.");
  let updated: NativeProviderProfile;
  try {
    const discovery = await native.discoverModels(selected.id, selected.endpoint);
    if (discovery.models.length === 0) {
      updated = { ...selected, discoveryError: "The provider returned no models." };
    } else {
      const hasKey = await native.hasKey(selected.id);
      updated = {
        ...selected,
        models: discovery.models,
        refreshedAt: discovery.refreshedAt,
        hasKey,
        discoveryError: null,
      };
    }
  } catch (error) {
    updated = {
      ...selected,
      discoveryError: error instanceof Error ? error.message : "Model discovery failed.",
    };
  }
  const profiles = current.map((profile) => profile.id === profileId ? updated : profile);
  await saveProfiles(context, profiles);
  return profiles;
}

export async function refreshNativeProviderKeyStatus(
  context: StorageContext,
  current: NativeProviderProfile[],
  profileId: string,
  hasKey: boolean,
): Promise<NativeProviderProfile[]> {
  const profiles = current.map((profile) => profile.id === profileId
    ? { ...profile, hasKey, discoveryError: null }
    : profile);
  await saveProfiles(context, profiles);
  return profiles;
}

export async function removeNativeProviderProfileMetadata(
  context: StorageContext,
  current: NativeProviderProfile[],
  profileId: string,
): Promise<NativeProviderProfile[]> {
  const profiles = current.filter((profile) => profile.id !== profileId);
  await saveProfiles(context, profiles);
  return profiles;
}

export async function deleteNativeProviderProfile(
  context: StorageContext,
  current: NativeProviderProfile[],
  profileId: string,
  native: DesktopAiProvidersBridge,
): Promise<NativeProviderProfile[]> {
  await native.deleteKey(profileId);
  return removeNativeProviderProfileMetadata(context, current, profileId);
}
