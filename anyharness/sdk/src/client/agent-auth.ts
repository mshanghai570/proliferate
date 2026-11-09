import type {
  CreateLocalProviderRequest,
  LocalProviderSummary,
  LocalProvidersResponse,
  AgentAuthStateDocument,
  ApplyAgentAuthStateResponse,
  NativeBridgeResponse,
} from "../types/agent-auth.js";
import type { AnyHarnessRequestOptions, AnyHarnessTransport } from "./core.js";

export class AgentAuthClient {
  constructor(private readonly transport: AnyHarnessTransport) {}

  listLocalProviders(options?: AnyHarnessRequestOptions): Promise<LocalProvidersResponse> {
    return this.transport.get("/v1/agent-auth/local-providers", options);
  }

  createLocalProvider(input: CreateLocalProviderRequest): Promise<LocalProviderSummary> {
    return this.transport.post("/v1/agent-auth/local-providers", input);
  }

  setLocalProviderEnabled(id: string, enabled: boolean): Promise<LocalProviderSummary> {
    return this.transport.patch(`/v1/agent-auth/local-providers/${encodeURIComponent(id)}`, { enabled });
  }

  removeLocalProvider(id: string): Promise<void> {
    return this.transport.delete(`/v1/agent-auth/local-providers/${encodeURIComponent(id)}`);
  }

  async applyState(
    input: AgentAuthStateDocument,
    options?: AnyHarnessRequestOptions,
  ): Promise<ApplyAgentAuthStateResponse> {
    return this.transport.put<ApplyAgentAuthStateResponse>(
      "/v1/agent-auth/state",
      input,
      options,
    );
  }

  async clearState(options?: AnyHarnessRequestOptions): Promise<void> {
    await this.transport.delete("/v1/agent-auth/state", options);
  }

  /** The native-migration bridge: harnesses still on the legacy flag. */
  async getNativeBridge(
    options?: AnyHarnessRequestOptions,
  ): Promise<NativeBridgeResponse> {
    return this.transport.get<NativeBridgeResponse>(
      "/v1/agent-auth/native-bridge",
      options,
    );
  }

  /**
   * The one-time prompt's dismiss-to-configure act: drop one harness's
   * legacy flag so its next launch follows the real convention.
   */
  async dismissNativeBridge(
    harnessKind: string,
    options?: AnyHarnessRequestOptions,
  ): Promise<void> {
    await this.transport.delete(
      `/v1/agent-auth/native-bridge/${encodeURIComponent(harnessKind)}`,
      options,
    );
  }
}
