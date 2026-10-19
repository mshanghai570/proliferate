import type { AgentRules, SetAgentRulesRequest } from "../types/agent-rules.js";
import type { AnyHarnessRequestOptions, AnyHarnessTransport } from "./core.js";

export class AgentRulesClient {
  constructor(private readonly transport: AnyHarnessTransport) {}
  get(kind: string, repositoryId: string | null, options?: AnyHarnessRequestOptions): Promise<AgentRules> {
    const query = repositoryId === null ? "" : `?repositoryId=${encodeURIComponent(repositoryId)}`;
    return this.transport.get(`/v1/agent-rules/${encodeURIComponent(kind)}${query}`, options);
  }
  set(kind: string, request: SetAgentRulesRequest): Promise<AgentRules> {
    return this.transport.put(`/v1/agent-rules/${encodeURIComponent(kind)}`, request);
  }
}
