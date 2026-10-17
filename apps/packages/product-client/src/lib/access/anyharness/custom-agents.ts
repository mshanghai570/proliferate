import type { AnyHarnessRequestOptions, RegisterCustomAcpAgentRequest } from "@anyharness/sdk";
import { getAnyHarnessClient, type AnyHarnessClientConnection } from "@anyharness/sdk-react";

export function listCustomAgents(connection: AnyHarnessClientConnection, options?: AnyHarnessRequestOptions) {
  return getAnyHarnessClient(connection).agents.listCustom(options);
}

export function registerCustomAgent(connection: AnyHarnessClientConnection, request: RegisterCustomAcpAgentRequest) {
  return getAnyHarnessClient(connection).agents.registerCustom(request);
}

export function removeCustomAgent(connection: AnyHarnessClientConnection, id: string) {
  return getAnyHarnessClient(connection).agents.removeCustom(id);
}

export function refreshCustomAgentModels(connection: AnyHarnessClientConnection, id: string) {
  return getAnyHarnessClient(connection).agents.refreshLaunchOptions(id);
}
