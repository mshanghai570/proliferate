import { getAnyHarnessClient, type AnyHarnessClientConnection } from "@anyharness/sdk-react";
export function getProjectRetrieval(connection: AnyHarnessClientConnection, workspace: string, signal?: AbortSignal) {
  return getAnyHarnessClient(connection).projectRetrieval.status(workspace, { signal });
}
export function indexProjectRetrieval(connection: AnyHarnessClientConnection, workspace: string) {
  return getAnyHarnessClient(connection).projectRetrieval.index(workspace);
}
export function clearProjectRetrieval(connection: AnyHarnessClientConnection, workspace: string) {
  return getAnyHarnessClient(connection).projectRetrieval.clear(workspace);
}
export function searchProjectContext(connection: AnyHarnessClientConnection, workspace: string, query: string) {
  return getAnyHarnessClient(connection).projectRetrieval.search(workspace, { query });
}
