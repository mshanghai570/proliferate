import type { ImportSkillRequest, ListSkillsResponse, SetSkillEnabledRequest, SkillSummary } from "../types/skills.js";
import type { AnyHarnessRequestOptions, AnyHarnessTransport } from "./core.js";

export class SkillsClient {
  constructor(private readonly transport: AnyHarnessTransport) {}
  list(options?: AnyHarnessRequestOptions): Promise<ListSkillsResponse> {
    return this.transport.get("/v1/skills", options);
  }
  import(request: ImportSkillRequest, options?: AnyHarnessRequestOptions): Promise<SkillSummary> {
    return this.transport.post("/v1/skills", request, options);
  }
  setEnabled(id: string, request: SetSkillEnabledRequest, options?: AnyHarnessRequestOptions): Promise<SkillSummary> {
    return this.transport.put(`/v1/skills/${encodeURIComponent(id)}/enabled`, request, options);
  }
  remove(id: string, options?: AnyHarnessRequestOptions): Promise<void> {
    return this.transport.delete(`/v1/skills/${encodeURIComponent(id)}`, options);
  }
}
