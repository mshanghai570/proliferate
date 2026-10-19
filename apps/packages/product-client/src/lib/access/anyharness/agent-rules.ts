import type { AnyHarnessClient } from "@anyharness/sdk";
import { getAnyHarnessClient, type AnyHarnessClientConnection } from "@anyharness/sdk-react";

type RulesWrite = Parameters<AnyHarnessClient["agentRules"]["set"]>[1];
export function getAgentRules(connection: AnyHarnessClientConnection, kind: string, repositoryId: string | null, signal?: AbortSignal) {
  return getAnyHarnessClient(connection).agentRules.get(kind, repositoryId, { signal });
}
export function setAgentRules(connection: AnyHarnessClientConnection, kind: string, request: RulesWrite) {
  return getAnyHarnessClient(connection).agentRules.set(kind, request);
}
