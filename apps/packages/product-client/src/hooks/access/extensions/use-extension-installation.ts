import { useMutation, useQueryClient } from "@tanstack/react-query";
import { resolveRuntimeCacheScopeKey, resolveRuntimeConnection, useAnyHarnessRuntimeContext } from "@anyharness/sdk-react";
import type { ExtensionPackageIdentity } from "@anyharness/sdk";
import { installRegisteredExtension } from "#product/lib/access/anyharness/extensions";

export function useExtensionInstallation() {
  const runtime = useAnyHarnessRuntimeContext();
  const queryClient = useQueryClient();
  const runtimeUrl = runtime.runtimeUrl?.trim() ?? "";
  const queryKey = ["anyharness", resolveRuntimeCacheScopeKey(runtime), runtimeUrl, "extensions"] as const;

  const install = useMutation({
    mutationFn: async (identity: ExtensionPackageIdentity) => {
      if (!runtimeUrl) throw new Error("No runtime available");
      return installRegisteredExtension(resolveRuntimeConnection(runtime), identity);
    },
    onMutate: () => ({ queryKey }),
    onSettled: (_data, _error, _identity, context) => {
      queryClient.invalidateQueries({ queryKey: context?.queryKey ?? queryKey });
    },
    retry: false,
    meta: { telemetryHandled: true },
  });

  return {
    install,
    runtimeAvailable: !!runtimeUrl,
  };
}
