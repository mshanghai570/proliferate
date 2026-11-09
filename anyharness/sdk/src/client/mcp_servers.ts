import type { AnyHarnessRequestOptions, AnyHarnessTransport } from "./core.js";
import type {
  CreateMcpServerRequest,
  ListMcpServersResponse,
  McpServerSummary,
  SetMcpServerEnabledRequest,
} from "../types/mcp-servers.js";

export type {
  CreateMcpServerRequest,
  ListMcpServersResponse,
  McpServerConfig,
  McpServerCredential,
  McpServerRepositoryOverride,
  McpServerSummary,
  McpServerTransport,
  SetMcpServerEnabledRequest,
} from "../types/mcp-servers.js";

export class McpServersClient {
  constructor(private readonly transport: AnyHarnessTransport) {}

  list(options?: AnyHarnessRequestOptions): Promise<ListMcpServersResponse> {
    return this.transport.get<ListMcpServersResponse>("/v1/mcp-servers", options);
  }

  create(
    request: CreateMcpServerRequest,
    options?: AnyHarnessRequestOptions,
  ): Promise<McpServerSummary> {
    return this.transport.post<McpServerSummary>("/v1/mcp-servers", request, options);
  }

  setEnabled(
    id: string,
    request: SetMcpServerEnabledRequest,
    options?: AnyHarnessRequestOptions,
  ): Promise<McpServerSummary> {
    return this.transport.put<McpServerSummary>(
      `/v1/mcp-servers/${encodeURIComponent(id)}/enabled`,
      request,
      options,
    );
  }

  remove(id: string, options?: AnyHarnessRequestOptions): Promise<void> {
    return this.transport.delete(`/v1/mcp-servers/${encodeURIComponent(id)}`, options);
  }
}
