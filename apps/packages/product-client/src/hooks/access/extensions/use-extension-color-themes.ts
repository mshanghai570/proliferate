import { useQuery } from "@tanstack/react-query";
import { resolveRuntimeCacheScopeKey, resolveRuntimeConnection, useAnyHarnessRuntimeContext } from "@anyharness/sdk-react";
import type { ExtensionPackageIdentity } from "@anyharness/sdk";
import { listExtensionColorThemes } from "#product/lib/access/anyharness/extensions";

export function useExtensionColorThemes(identity: ExtensionPackageIdentity, enabled: boolean) {
  const runtime = useAnyHarnessRuntimeContext();
  return useQuery({
    queryKey: ["anyharness", resolveRuntimeCacheScopeKey(runtime), runtime.runtimeUrl?.trim() ?? "", "extensions", identity.namespace, identity.name, identity.version, "color-themes"],
    enabled: enabled && !!runtime.runtimeUrl?.trim(),
    queryFn: ({ signal }) => listExtensionColorThemes(resolveRuntimeConnection(runtime), identity, { signal }),
    staleTime: 15_000,
    retry: false,
    refetchOnWindowFocus: false,
    meta: { telemetryHandled: true },
  });
}
