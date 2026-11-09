import type { RegisterCustomAcpAgentRequest } from "@anyharness/sdk";
import { resolveRuntimeConnection, useAnyHarnessCacheScopeKey, useAnyHarnessRuntimeContext } from "@anyharness/sdk-react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { listCustomAgents, refreshCustomAgentModels, registerCustomAgent, removeCustomAgent } from "#product/lib/access/anyharness/custom-agents";
import { useAgentResourcesCache } from "#product/hooks/access/anyharness/agents/use-agent-resources-cache";

export function useCustomAgents(enabled: boolean) {
  const runtime = useAnyHarnessRuntimeContext();
  const cacheScopeKey = useAnyHarnessCacheScopeKey();
  const runtimeUrl = runtime.runtimeUrl?.trim() ?? "";
  const connection = () => resolveRuntimeConnection(runtime);
  const queryClient = useQueryClient();
  const cache = useAgentResourcesCache();
  const queryKey = ["anyharness", cacheScopeKey, runtimeUrl, "custom-agents"];
  const query = useQuery({
    queryKey,
    enabled: enabled && Boolean(runtimeUrl),
    queryFn: ({ signal }) => listCustomAgents(connection(), { signal }),
  });
  async function invalidate() {
    await Promise.all([
      queryClient.invalidateQueries({ queryKey }),
      cache.invalidateAgentLaunchReadinessResources(runtimeUrl),
    ]);
  }
  const register = useMutation({
    mutationFn: (request: RegisterCustomAcpAgentRequest) => registerCustomAgent(connection(), request),
    onSuccess: invalidate,
    retry: false,
  });
  const remove = useMutation({
    mutationFn: (id: string) => removeCustomAgent(connection(), id),
    onSuccess: invalidate,
    retry: false,
  });
  const refresh = useMutation({
    mutationFn: (id: string) => refreshCustomAgentModels(connection(), id),
    onSuccess: invalidate,
    retry: false,
  });
  return { query, register, remove, refresh, runtimeAvailable: Boolean(runtimeUrl) };
}
