import type { AnyHarnessRequestOptions, SessionMcpServer } from "@anyharness/sdk";
import { getAnyHarnessClient, type AnyHarnessClientConnection } from "@anyharness/sdk-react";

export function listProjectMcpServers(connection: AnyHarnessClientConnection, repo: string, options?: AnyHarnessRequestOptions) {
  return getAnyHarnessClient(connection).repoRoots.listMcpServers(repo, options);
}
export function saveProjectMcpServer(connection: AnyHarnessClientConnection, repo: string, name: string, server: SessionMcpServer) {
  return getAnyHarnessClient(connection).repoRoots.putMcpServer(repo, name, server);
}
export function setProjectMcpServerEnabled(connection: AnyHarnessClientConnection, repo: string, name: string, enabled: boolean) {
  return getAnyHarnessClient(connection).repoRoots.setMcpServerEnabled(repo, name, enabled);
}
export function removeProjectMcpServer(connection: AnyHarnessClientConnection, repo: string, name: string) {
  return getAnyHarnessClient(connection).repoRoots.removeMcpServer(repo, name);
}
