import type {
  CreateMcpServerRequest,
  SetMcpServerEnabledRequest,
} from "@anyharness/sdk";
import { getAnyHarnessClient, type AnyHarnessClientConnection } from "@anyharness/sdk-react";

export function listMcpServers(connection: AnyHarnessClientConnection, signal?: AbortSignal) {
  return getAnyHarnessClient(connection).mcpServers.list({ signal });
}

export function createMcpServer(
  connection: AnyHarnessClientConnection,
  request: CreateMcpServerRequest,
) {
  return getAnyHarnessClient(connection).mcpServers.create(request);
}

export function setMcpServerEnabled(
  connection: AnyHarnessClientConnection,
  id: string,
  request: SetMcpServerEnabledRequest,
) {
  return getAnyHarnessClient(connection).mcpServers.setEnabled(id, request);
}

export function removeMcpServer(connection: AnyHarnessClientConnection, id: string) {
  return getAnyHarnessClient(connection).mcpServers.remove(id);
}
