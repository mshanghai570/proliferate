import type { ProjectRetrievalStatus, ProjectContextSearchResponse, SearchProjectContextRequest } from "../types/project-retrieval.js";
import type { AnyHarnessRequestOptions, AnyHarnessTransport } from "./core.js";
export class ProjectRetrievalClient {
  constructor(private readonly transport: AnyHarnessTransport) {}
  status(workspaceId: string, options?: AnyHarnessRequestOptions): Promise<ProjectRetrievalStatus> {
    return this.transport.get(`/v1/workspaces/${encodeURIComponent(workspaceId)}/retrieval`, options);
  }
  index(workspaceId: string): Promise<ProjectRetrievalStatus> {
    return this.transport.post(`/v1/workspaces/${encodeURIComponent(workspaceId)}/retrieval`, {});
  }
  clear(workspaceId: string): Promise<void> {
    return this.transport.delete(`/v1/workspaces/${encodeURIComponent(workspaceId)}/retrieval`);
  }
  search(workspaceId: string, request: SearchProjectContextRequest): Promise<ProjectContextSearchResponse> {
    return this.transport.post(`/v1/workspaces/${encodeURIComponent(workspaceId)}/retrieval/search`, request);
  }
}
