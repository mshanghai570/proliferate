import type {
  AgentSummary,
  CustomAcpAgent,
  RegisterCustomAcpAgentRequest,
  HarnessLaunchOptionsResponse,
  InstallAgentRequest,
  InstallAgentResponse,
  ReconcileAgentsRequest,
  ReconcileAgentsResponse,
  AgentLoginTerminalRecord,
  AgentLoginVariant,
  ClaimAgentMintTokenResponse,
  StartAgentLoginResponse,
  StartAgentLoginTerminalResponse,
} from "../types/agents.js";
import type { AnyHarnessRequestOptions, AnyHarnessTransport } from "./core.js";

export class AgentsClient {
  constructor(private readonly transport: AnyHarnessTransport) {}

  async listCustom(options?: AnyHarnessRequestOptions): Promise<CustomAcpAgent[]> {
    return this.transport.get<CustomAcpAgent[]>("/v1/agents/custom", options);
  }

  async registerCustom(request: RegisterCustomAcpAgentRequest): Promise<CustomAcpAgent> {
    return this.transport.post<CustomAcpAgent>("/v1/agents/custom", request);
  }

  async removeCustom(id: string): Promise<void> {
    return this.transport.delete(`/v1/agents/custom/${encodeURIComponent(id)}`);
  }

  async list(options?: AnyHarnessRequestOptions): Promise<AgentSummary[]> {
    return this.transport.get<AgentSummary[]>("/v1/agents", options);
  }

  async get(kind: string, options?: AnyHarnessRequestOptions): Promise<AgentSummary> {
    return this.transport.get<AgentSummary>(
      `/v1/agents/${encodeURIComponent(kind)}`,
      options,
    );
  }

  async getLaunchOptions(
    harnessKind: string,
    options?: AnyHarnessRequestOptions,
  ): Promise<HarnessLaunchOptionsResponse> {
    return this.transport.get<HarnessLaunchOptionsResponse>(
      `/v1/agents/${encodeURIComponent(harnessKind)}/launch-options`,
      options,
    );
  }

  async refreshLaunchOptions(harnessKind: string): Promise<HarnessLaunchOptionsResponse> {
    return this.transport.post<HarnessLaunchOptionsResponse>(
      `/v1/agents/${encodeURIComponent(harnessKind)}/launch-options/refresh`,
      {},
    );
  }

  async install(
    kind: string,
    request: InstallAgentRequest = {},
  ): Promise<InstallAgentResponse> {
    return this.transport.post<InstallAgentResponse>(
      `/v1/agents/${encodeURIComponent(kind)}/install`,
      request,
    );
  }

  async startLogin(kind: string): Promise<StartAgentLoginResponse> {
    return this.transport.post<StartAgentLoginResponse>(
      `/v1/agents/${encodeURIComponent(kind)}/login/start`,
      {},
    );
  }

  async startLoginTerminal(
    kind: string,
    variant?: AgentLoginVariant,
  ): Promise<StartAgentLoginTerminalResponse> {
    return this.transport.post<StartAgentLoginTerminalResponse>(
      `/v1/agents/${encodeURIComponent(kind)}/login/terminal`,
      variant ? { variant } : {},
    );
  }

  /**
   * The one-time seat-token handoff (seats v1): returns the captured mint
   * token exactly once — the runtime wipes its buffer as it serves this.
   * Callers hold the token in memory only and POST it straight to the vault.
   */
  async claimMintToken(terminalId: string): Promise<ClaimAgentMintTokenResponse> {
    return this.transport.post<ClaimAgentMintTokenResponse>(
      `/v1/agents/login-terminals/${encodeURIComponent(terminalId)}/mint-token`,
      {},
    );
  }

  async getLoginTerminal(
    terminalId: string,
    options?: AnyHarnessRequestOptions,
  ): Promise<AgentLoginTerminalRecord> {
    return this.transport.get<AgentLoginTerminalRecord>(
      `/v1/agents/login-terminals/${encodeURIComponent(terminalId)}`,
      options,
    );
  }

  async closeLoginTerminal(terminalId: string): Promise<void> {
    return this.transport.delete(
      `/v1/agents/login-terminals/${encodeURIComponent(terminalId)}`,
    );
  }

  async getReconcileStatus(options?: AnyHarnessRequestOptions): Promise<ReconcileAgentsResponse> {
    return this.transport.get<ReconcileAgentsResponse>("/v1/agents/reconcile", options);
  }

  async reconcile(
    request: ReconcileAgentsRequest = {},
  ): Promise<ReconcileAgentsResponse> {
    return this.transport.post<ReconcileAgentsResponse>(
      "/v1/agents/reconcile",
      request,
    );
  }
}
