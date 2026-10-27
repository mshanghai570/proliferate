import { beforeEach, describe, expect, it, vi } from "vitest";
import type { DesktopAiProvidersBridge } from "@proliferate/product-client/host/desktop-bridge";
import type { ProductStorage } from "@proliferate/product-client/host/product-host";
import {
  addNativeProviderProfile,
  deleteNativeProviderProfile,
  discoverNativeProviderModels,
  loadNativeProviderProfiles,
  NATIVE_PROVIDER_PROFILES_STORAGE_KEY,
  updateNativeProviderProfile,
  type NativeProviderProfile,
} from "#product/lib/domain/agents/native-provider-profiles";

const PROFILE: NativeProviderProfile = {
  id: "0f1e2d3c-4b5a-6978-8f0e-1d2c3b4a5968",
  name: "Local model server",
  endpoint: "http://localhost:1234/v1",
  models: [{ id: "model-a" }],
  refreshedAt: "2026-10-23T10:00:00.000Z",
  hasKey: false,
  discoveryError: null,
};

function storageWith(value: string | null = null) {
  const data = new Map<string, string>();
  if (value !== null) data.set(NATIVE_PROVIDER_PROFILES_STORAGE_KEY, value);
  const storage: ProductStorage = {
    getItem: vi.fn(async (key) => data.get(key) ?? null),
    setItem: vi.fn(async (key, next) => { data.set(key, next); }),
    removeItem: vi.fn(async (key) => { data.delete(key); }),
  };
  return { storage, data, captureException: vi.fn() };
}

beforeEach(() => vi.restoreAllMocks());

describe("native provider profile persistence", () => {
  it("loads valid local metadata and ignores malformed records", async () => {
    const context = storageWith(JSON.stringify([PROFILE, { ...PROFILE, id: "bad-id" }, { name: "incomplete" }]));
    expect(await loadNativeProviderProfiles(context)).toEqual([PROFILE]);
  });

  it("adds a normalized profile without persisting a provider key", async () => {
    const context = storageWith();
    vi.spyOn(globalThis.crypto, "randomUUID").mockReturnValue(PROFILE.id);
    const profiles = await addNativeProviderProfile(context, [], {
      name: "  Local model server ", endpoint: " http://localhost:1234/v1/ ",
    });
    expect(profiles).toEqual([{ ...PROFILE, models: [], refreshedAt: null }]);
    const stored = context.data.get(NATIVE_PROVIDER_PROFILES_STORAGE_KEY) ?? "";
    expect(stored).not.toContain("apiKey");
    expect(stored).not.toContain("secret");
  });

  it("rejects invalid endpoints without storing a profile", async () => {
    const context = storageWith();
    await expect(addNativeProviderProfile(context, [], {
      name: "Remote", endpoint: "https://user:secret@example.test/v1",
    })).rejects.toThrow(/credentials/i);
    expect(context.storage.setItem).not.toHaveBeenCalled();
  });

  it("updates a profile and clears its previous discovery cache when the endpoint changes", async () => {
    const context = storageWith();
    const profiles = await updateNativeProviderProfile(context, [PROFILE], PROFILE.id, {
      name: "Renamed endpoint", endpoint: "https://api.example.test/v1",
    });
    expect(profiles[0]).toMatchObject({
      name: "Renamed endpoint", endpoint: "https://api.example.test/v1",
      models: [], refreshedAt: null, hasKey: false,
    });
  });

  it("deletes the Keychain credential before removing profile metadata", async () => {
    const context = storageWith();
    const native: DesktopAiProvidersBridge = {
      isSupported: () => true,
      promptForKey: vi.fn(async () => "saved"),
      hasKey: vi.fn(async () => false),
      deleteKey: vi.fn(async () => {}),
      discoverModels: vi.fn(async () => ({ models: [], refreshedAt: "" })),
    };
    await deleteNativeProviderProfile(context, [PROFILE], PROFILE.id, native);
    expect(native.deleteKey).toHaveBeenCalledWith(PROFILE.id);
    expect(context.data.get(NATIVE_PROVIDER_PROFILES_STORAGE_KEY)).toBe("[]");
  });
});

describe("native provider model discovery", () => {
  it("sends profile id and endpoint to native mediation and replaces cache on success", async () => {
    const context = storageWith();
    const native: DesktopAiProvidersBridge = {
      isSupported: () => true,
      promptForKey: vi.fn(async () => "saved"),
      hasKey: vi.fn(async () => true),
      deleteKey: vi.fn(async () => {}),
      discoverModels: vi.fn(async () => ({
        models: [{ id: "model-b" }, { id: "org/model-c" }], refreshedAt: "2026-10-23T11:00:00.000Z",
      })),
    };
    const profiles = await discoverNativeProviderModels(context, [PROFILE], PROFILE.id, native);
    expect(native.discoverModels).toHaveBeenCalledWith(PROFILE.id, PROFILE.endpoint);
    expect(profiles[0]).toMatchObject({
      models: [{ id: "model-b" }, { id: "org/model-c" }],
      refreshedAt: "2026-10-23T11:00:00.000Z", discoveryError: null, hasKey: true,
    });
  });

  it("keeps cached models and marks discovery stale when native discovery fails", async () => {
    const context = storageWith();
    const native: DesktopAiProvidersBridge = {
      isSupported: () => true,
      promptForKey: vi.fn(async () => "saved"),
      hasKey: vi.fn(async () => true),
      deleteKey: vi.fn(async () => {}),
      discoverModels: vi.fn(async () => { throw new Error("Provider timed out."); }),
    };
    const profiles = await discoverNativeProviderModels(context, [PROFILE], PROFILE.id, native);
    expect(profiles[0]).toMatchObject({
      models: PROFILE.models, refreshedAt: PROFILE.refreshedAt, discoveryError: "Provider timed out.",
    });
    expect(context.data.get(NATIVE_PROVIDER_PROFILES_STORAGE_KEY)).toContain("model-a");
  });

  it("does not overwrite model cache when discovery returns an empty list", async () => {
    const context = storageWith();
    const native: DesktopAiProvidersBridge = {
      isSupported: () => true,
      promptForKey: vi.fn(async () => "saved"),
      hasKey: vi.fn(async () => false),
      deleteKey: vi.fn(async () => {}),
      discoverModels: vi.fn(async () => ({ models: [], refreshedAt: "2026-10-23T12:00:00Z" })),
    };
    const profiles = await discoverNativeProviderModels(context, [PROFILE], PROFILE.id, native);
    expect(profiles[0]).toMatchObject({ models: PROFILE.models, discoveryError: "The provider returned no models." });
  });
});
