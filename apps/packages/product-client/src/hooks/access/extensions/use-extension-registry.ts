import { useQuery } from "@tanstack/react-query";
import { useExtensionRegistry } from "#product/providers/ExtensionRegistryProvider";
import { fetchExtensionContent } from "#product/lib/access/extensions/extension-content";
import { ExtensionRegistryError } from "#product/lib/domain/extensions/extension-registry-error";
import { parseExtensionManifest } from "#product/lib/domain/extensions/extension-manifest";
import type { ExtensionRef, ExtensionSearchQuery } from "#product/lib/domain/extensions/extension-models";

const queryPolicy = {
  staleTime: 60_000,
  gcTime: 5 * 60_000,
  retry: false,
  refetchOnWindowFocus: false,
  // Registry content and user search terms must not become exception payloads.
  meta: { telemetryHandled: true },
} as const;

export function useExtensionSearch(query: ExtensionSearchQuery) {
  const registry = useExtensionRegistry();
  return useQuery({
    ...queryPolicy,
    queryKey: ["extensions", "open-vsx", "search", query],
    queryFn: ({ signal }) => registry.search(query, { signal }),
  });
}

export function useExtensionMetadata(ref: ExtensionRef | null) {
  const registry = useExtensionRegistry();
  return useQuery({
    ...queryPolicy,
    queryKey: ["extensions", "open-vsx", "metadata", ref],
    enabled: ref !== null,
    queryFn: ({ signal }) => {
      if (!ref) throw new Error("Extension identity is required");
      return registry.getExtension(ref, { signal });
    },
  });
}

export function useExtensionDocument(url: string | null) {
  return useQuery({
    ...queryPolicy,
    queryKey: ["extensions", "document", url],
    enabled: url !== null,
    queryFn: ({ signal }) => fetchExtensionContent(url!, { signal }),
  });
}

export function useExtensionManifest(url: string | null) {
  return useQuery({
    ...queryPolicy,
    queryKey: ["extensions", "manifest", url],
    enabled: url !== null,
    queryFn: async ({ signal }) => {
      const text = await fetchExtensionContent(url!, { signal });
      try {
        return parseExtensionManifest(JSON.parse(text), url!);
      } catch (error) {
        if (error instanceof SyntaxError) throw new ExtensionRegistryError("malformed-response", "Invalid extension manifest JSON");
        throw error;
      }
    },
  });
}
