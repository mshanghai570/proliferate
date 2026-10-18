import { useQuery, useQueryClient } from "@tanstack/react-query";
import { resolveRuntimeCacheScopeKey, resolveRuntimeConnection, useAnyHarnessRuntimeContext } from "@anyharness/sdk-react";
import type { CreateLocalProviderRequest, LocalProviderSummary, LocalProvidersResponse } from "@anyharness/sdk";
import { listLocalProviders, createLocalProvider, setLocalProviderEnabled, removeLocalProvider } from "#product/lib/access/anyharness/local-providers";

export function useLocalProviders() {
  const runtime = useAnyHarnessRuntimeContext();
  const client = useQueryClient();
  const identity = JSON.stringify([resolveRuntimeCacheScopeKey(runtime), runtime.runtimeUrl?.trim() ?? ""]);
  const queryKey = ["anyharness", identity, "local-providers"];
  const query = useQuery({ queryKey, enabled: !!runtime.runtimeUrl?.trim(),
    queryFn: ({ signal }) => listLocalProviders(resolveRuntimeConnection(runtime), { signal }),
    retry: false, refetchOnWindowFocus: false, meta: { telemetryHandled: true },
  });
  async function update(id: string | null, action: () => Promise<LocalProviderSummary | void>) {
    const saved = await action();
    client.setQueryData<LocalProvidersResponse>(queryKey, (current) => {
      if (!current) return current;
      const changedId = saved ? saved.id : id;
      const providers = current.providers.filter((row) => row.id !== changedId);
      if (saved) providers.push(saved);
      return { ...current, providers };
    });
    void client.invalidateQueries({ queryKey, exact: true });
  }
  // Credentials are write-only and never enter React Query's mutation cache.
  return { identity, query, connected: !!runtime.runtimeUrl?.trim(),
    save: (input: CreateLocalProviderRequest) => update(null, () => createLocalProvider(resolveRuntimeConnection(runtime), input)),
    toggle: (id: string, enabled: boolean) => update(id, () => setLocalProviderEnabled(resolveRuntimeConnection(runtime), id, enabled)),
    remove: (id: string) => update(id, () => removeLocalProvider(resolveRuntimeConnection(runtime), id)),
  };
}
