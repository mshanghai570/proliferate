import { invoke } from "@tauri-apps/api/core";
import type {
  DesktopAiProviderDiscovery,
} from "@proliferate/product-client/host/desktop-bridge";

export async function promptAiProviderKey(
  profileId: string,
): Promise<"saved" | "cancelled"> {
  const result = await invoke<unknown>("prompt_ai_provider_key", { profileId });
  if (result !== "saved" && result !== "cancelled") {
    throw new Error("Invalid provider-key prompt response.");
  }
  return result;
}

export async function hasAiProviderKey(profileId: string): Promise<boolean> {
  const result = await invoke<unknown>("has_ai_provider_key", { profileId });
  if (typeof result !== "boolean") {
    throw new Error("Invalid provider-key status response.");
  }
  return result;
}

export async function deleteAiProviderKey(profileId: string): Promise<void> {
  await invoke("delete_ai_provider_key", { profileId });
}

export async function discoverAiProviderModels(
  profileId: string,
  endpoint: string,
): Promise<DesktopAiProviderDiscovery> {
  const result = await invoke<unknown>("discover_ai_provider_models", { profileId, endpoint });
  if (!isDiscovery(result)) {
    throw new Error("The local runtime returned an invalid model list.");
  }
  return result;
}

function isDiscovery(value: unknown): value is DesktopAiProviderDiscovery {
  if (!value || typeof value !== "object") return false;
  const record = value as Record<string, unknown>;
  return Array.isArray(record.models)
    && record.models.every((model) => model !== null
      && typeof model === "object"
      && typeof (model as Record<string, unknown>).id === "string"
      && ((model as Record<string, unknown>).displayName === undefined
        || typeof (model as Record<string, unknown>).displayName === "string"))
    && typeof record.refreshedAt === "string";
}
