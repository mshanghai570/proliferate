import { useQuery, useQueryClient } from "@tanstack/react-query";
import { resolveRuntimeCacheScopeKey, resolveRuntimeConnection, useAnyHarnessRuntimeContext } from "@anyharness/sdk-react";
import type { ProjectMcpServersResponse, ProjectMcpServerSummary, SessionMcpServer } from "@anyharness/sdk";
import { listProjectMcpServers, saveProjectMcpServer, setProjectMcpServerEnabled, removeProjectMcpServer } from "#product/lib/access/anyharness/project-mcp";

export function useProjectMcpServers(repo: string) {
  const runtime = useAnyHarnessRuntimeContext();
  const client = useQueryClient();
  const queryKey = ["anyharness", resolveRuntimeCacheScopeKey(runtime), runtime.runtimeUrl?.trim() ?? "", "project-mcp", repo];
  const query = useQuery({
    queryKey,
    enabled: !!runtime.runtimeUrl?.trim(),
    queryFn: ({ signal }) => listProjectMcpServers(resolveRuntimeConnection(runtime), repo, { signal }),
    retry: false,
    refetchOnWindowFocus: false,
    meta: { telemetryHandled: true },
  });
  async function update(name: string, action: () => Promise<ProjectMcpServerSummary | void>) {
    const saved = await action();
    client.setQueryData<ProjectMcpServersResponse>(queryKey, (current) => {
      if (!current) return current;
      const servers = current.servers.filter((server) => server.name !== name);
      if (saved) servers.push(saved);
      servers.sort((a, b) => a.name < b.name ? -1 : a.name > b.name ? 1 : 0);
      return { ...current, servers };
    });
    void client.invalidateQueries({ queryKey, exact: true });
  }
  // Avoid mutation-cache retention of write-only credentials.
  return {
    query,
    connected: !!runtime.runtimeUrl?.trim(),
    save: (name: string, server: SessionMcpServer) => update(name, () => saveProjectMcpServer(resolveRuntimeConnection(runtime), repo, name, server)),
    toggle: (name: string, enabled: boolean) => update(name, () => setProjectMcpServerEnabled(resolveRuntimeConnection(runtime), repo, name, enabled)),
    remove: (name: string) => update(name, () => removeProjectMcpServer(resolveRuntimeConnection(runtime), repo, name)),
  };
}
