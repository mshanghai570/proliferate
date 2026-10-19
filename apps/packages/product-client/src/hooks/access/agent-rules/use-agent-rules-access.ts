import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { resolveRuntimeCacheScopeKey, resolveRuntimeConnection, useAnyHarnessRuntimeContext } from "@anyharness/sdk-react";
import { getAgentRules, setAgentRules } from "#product/lib/access/anyharness/agent-rules";

export function useAgentRulesAccess(kind: string, repositoryId: string | null) {
  const runtime = useAnyHarnessRuntimeContext();
  const available = !!runtime.runtimeUrl?.trim();
  const queryKey = ["anyharness", resolveRuntimeCacheScopeKey(runtime), runtime.runtimeUrl, "agent-rules", kind, repositoryId] as const;
  const client = useQueryClient();
  const query = useQuery({ queryKey, enabled: available && !!kind, retry: false,
    queryFn: ({ signal }) => getAgentRules(resolveRuntimeConnection(runtime), kind, repositoryId, signal),
  });
  const mutation = useMutation({ retry: false,
    mutationFn: (text: string | null) => setAgentRules(resolveRuntimeConnection(runtime), kind, { repositoryId, text }),
    onSuccess: (data) => client.setQueryData(queryKey, data),
    onSettled: () => client.invalidateQueries({ queryKey: queryKey.slice(0, 4) }),
  });
  return { query, mutation, available };
}
