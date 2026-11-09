import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { resolveRuntimeCacheScopeKey, resolveRuntimeConnection, useAnyHarnessRuntimeContext, type AnyHarnessClientConnection } from "@anyharness/sdk-react";
import type { ExtensionPackageIdentity } from "@anyharness/sdk";
import { listInstalledExtensions, uninstallRegisteredExtension } from "#product/lib/access/anyharness/extensions";

export interface UninstallTarget {
  identity: ExtensionPackageIdentity;
  connection: AnyHarnessClientConnection;
  queryKey: readonly string[];
  scope: string;
}

export function useInstalledExtensionsAccess() {
  const runtime = useAnyHarnessRuntimeContext();
  const runtimeUrl = runtime.runtimeUrl?.trim() ?? "";
  const queryKey = ["anyharness", resolveRuntimeCacheScopeKey(runtime), runtimeUrl, "extensions"] as const;
  const scope = JSON.stringify(queryKey);
  const queryClient = useQueryClient();
  const query = useQuery({
    queryKey,
    enabled: !!runtimeUrl,
    queryFn: ({ signal }) => listInstalledExtensions(resolveRuntimeConnection(runtime), { signal }),
    staleTime: 15_000,
    retry: false,
    refetchOnWindowFocus: false,
    meta: { telemetryHandled: true },
  });
  const uninstall = useMutation({
    mutationFn: (target: UninstallTarget) => uninstallRegisteredExtension(target.connection, target.identity),
    onSettled: (_data, _error, target) => queryClient.invalidateQueries({ queryKey: target.queryKey }),
    retry: false,
    meta: { telemetryHandled: true },
  });
  return {
    query, uninstall, scope, runtimeAvailable: !!runtimeUrl,
    uninstallTarget: (identity: ExtensionPackageIdentity): UninstallTarget => ({ identity, connection: resolveRuntimeConnection(runtime), queryKey, scope }),
  };
}
