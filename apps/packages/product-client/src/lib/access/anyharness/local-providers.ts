import { getAnyHarnessClient, type AnyHarnessClientConnection } from "@anyharness/sdk-react";
import type { AnyHarnessRequestOptions, CreateLocalProviderRequest } from "@anyharness/sdk";

export function listLocalProviders(connection: AnyHarnessClientConnection, options?: AnyHarnessRequestOptions) {
  return getAnyHarnessClient(connection).agentAuth.listLocalProviders(options);
}
export function createLocalProvider(connection: AnyHarnessClientConnection, input: CreateLocalProviderRequest) {
  return getAnyHarnessClient(connection).agentAuth.createLocalProvider(input);
}
export function setLocalProviderEnabled(connection: AnyHarnessClientConnection, id: string, enabled: boolean) {
  return getAnyHarnessClient(connection).agentAuth.setLocalProviderEnabled(id, enabled);
}
export function removeLocalProvider(connection: AnyHarnessClientConnection, id: string) {
  return getAnyHarnessClient(connection).agentAuth.removeLocalProvider(id);
}
