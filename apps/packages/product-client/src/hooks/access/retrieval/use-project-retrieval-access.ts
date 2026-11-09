import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { resolveRuntimeCacheScopeKey, resolveRuntimeConnection, useAnyHarnessRuntimeContext } from "@anyharness/sdk-react";
import { getProjectRetrieval, indexProjectRetrieval, clearProjectRetrieval, searchProjectContext } from "#product/lib/access/anyharness/project-retrieval";
export function useProjectRetrievalAccess(workspace: string) {
  const runtime = useAnyHarnessRuntimeContext();
  const available = !!runtime.runtimeUrl?.trim() && !!workspace;
  const connection = () => resolveRuntimeConnection(runtime);
  const queryKey = ["anyharness", resolveRuntimeCacheScopeKey(runtime), runtime.runtimeUrl, "project-retrieval", workspace] as const;
  const client = useQueryClient();
  const query = useQuery({ queryKey, enabled: available, retry: false,
    queryFn: ({ signal }) => getProjectRetrieval(connection(), workspace, signal),
  });
  const refresh = useMutation({ retry: false,
    mutationFn: () => indexProjectRetrieval(connection(), workspace),
    onSuccess: (data) => client.setQueryData(queryKey, data),
    onSettled: () => client.invalidateQueries({ queryKey }),
  });
  const clear = useMutation({ retry: false,
    mutationFn: () => clearProjectRetrieval(connection(), workspace),
    onSettled: () => client.invalidateQueries({ queryKey }),
  });
  const search = useMutation({ retry: false,
    mutationFn: (text: string) => searchProjectContext(connection(), workspace, text),
    onSuccess: (data) => client.setQueryData(queryKey, data.status),
  });
  return { query, refresh, clear, search, available };
}
