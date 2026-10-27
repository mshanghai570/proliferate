import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import {
  resolveRuntimeCacheScopeKey,
  resolveRuntimeConnection,
  useAnyHarnessRuntimeContext,
} from "@anyharness/sdk-react";
import type {
  CreateMcpServerRequest,
  SetMcpServerEnabledRequest,
} from "@anyharness/sdk";
import {
  createMcpServer,
  listMcpServers,
  removeMcpServer,
  setMcpServerEnabled,
} from "#product/lib/access/anyharness/mcp-servers";

type Action =
  | { kind: "create"; request: CreateMcpServerRequest }
  | { kind: "enable"; id: string; request: SetMcpServerEnabledRequest }
  | { kind: "remove"; id: string };

export function useMcpServersAccess() {
  const runtime = useAnyHarnessRuntimeContext();
  const available = !!runtime.runtimeUrl?.trim();
  const connection = () => {
    if (!available) throw new Error("Runtime unavailable");
    return resolveRuntimeConnection(runtime);
  };
  const queryKey = [
    "anyharness",
    resolveRuntimeCacheScopeKey(runtime),
    runtime.runtimeUrl,
    "mcp-servers",
  ] as const;
  const client = useQueryClient();
  const query = useQuery({
    queryKey,
    enabled: available,
    queryFn: ({ signal }) => listMcpServers(connection(), signal),
    retry: false,
  });
  const mutation = useMutation<unknown, Error, Action>({
    mutationFn: (action) => {
      if (action.kind === "create") return createMcpServer(connection(), action.request);
      if (action.kind === "enable") return setMcpServerEnabled(connection(), action.id, action.request);
      return removeMcpServer(connection(), action.id);
    },
    onSettled: () => client.invalidateQueries({ queryKey }),
    retry: false,
  });

  return { query, mutation, available };
}
