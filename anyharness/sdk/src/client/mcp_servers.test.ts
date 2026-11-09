import { describe, expect, it, vi } from "vitest";
import { AnyHarnessClient } from "./core.js";
import type { CreateMcpServerRequest, McpServerSummary } from "./mcp_servers.js";

const summary: McpServerSummary = {
  id: "mcp-1",
  serverName: "filesystem",
  transport: "stdio",
  envNames: ["FILESYSTEM_TOKEN"],
  headerNames: [],
  userEnabled: null,
  repositoryOverrides: {},
  createdAt: "2026-10-26T00:00:00Z",
  updatedAt: "2026-10-26T00:00:00Z",
};

describe("McpServersClient", () => {
  it("lists typed server metadata from the runtime", async () => {
    const response = { servers: [summary] };
    const fetch = vi.fn(async () => new Response(JSON.stringify(response)));
    const client = new AnyHarnessClient({ baseUrl: "http://runtime.test/", fetch });

    await expect(client.mcpServers.list()).resolves.toEqual(response);
    const [url, options] = fetch.mock.calls[0] as unknown as [string, RequestInit];
    expect(url).toBe("http://runtime.test/v1/mcp-servers");
    expect(options.method).toBe("GET");
  });

  it("posts stdio and HTTP create payloads and returns typed summaries", async () => {
    const request: CreateMcpServerRequest = {
      serverName: "filesystem",
      description: "Workspace files",
      config: {
        transport: "stdio",
        command: "mcp-filesystem",
        args: ["/workspace"],
        env: [{ name: "TOKEN", value: "secret-value" }],
      },
    };
    const fetch = vi.fn(async () => new Response(JSON.stringify(summary), { status: 201 }));
    const client = new AnyHarnessClient({ baseUrl: "http://runtime.test", fetch });

    await expect(client.mcpServers.create(request)).resolves.toEqual(summary);
    const [url, options] = fetch.mock.calls[0] as unknown as [string, RequestInit];
    expect(url).toBe("http://runtime.test/v1/mcp-servers");
    expect(options.method).toBe("POST");
    expect(JSON.parse(String(options.body))).toEqual(request);
  });

  it("serializes the tagged HTTP config and header credential", async () => {
    const request: CreateMcpServerRequest = {
      serverName: "remote",
      config: {
        transport: "http",
        url: "https://mcp.example.test/api",
        headers: [{ name: "Authorization", value: "secret-value" }],
      },
    };
    const fetch = vi.fn(async () => new Response(JSON.stringify({ ...summary, transport: "http" }), { status: 201 }));
    const client = new AnyHarnessClient({ baseUrl: "http://runtime.test", fetch });

    await client.mcpServers.create(request);
    const [, options] = fetch.mock.calls[0] as unknown as [string, RequestInit];
    expect(JSON.parse(String(options.body))).toEqual(request);
  });

  it("sends user-default booleans and exact repository override/reset bodies", async () => {
    const responses = [summary, summary, summary];
    const fetch = vi.fn(async () => new Response(JSON.stringify(responses.shift())));
    const client = new AnyHarnessClient({ baseUrl: "http://runtime.test", fetch });

    await client.mcpServers.setEnabled("mcp/1", true);
    await client.mcpServers.setEnabled("mcp/1", { repoRootId: "repo-1", enabled: false });
    await client.mcpServers.setEnabled("mcp/1", { repoRootId: "repo-1", enabled: null });
    expect(fetch.mock.calls.map(([url, options]) => ({
      url,
      method: (options as RequestInit).method,
      body: JSON.parse(String((options as RequestInit).body)),
    }))).toEqual([
      { url: "http://runtime.test/v1/mcp-servers/mcp%2F1/enabled", method: "PUT", body: true },
      { url: "http://runtime.test/v1/mcp-servers/mcp%2F1/enabled", method: "PUT", body: { repoRootId: "repo-1", enabled: false } },
      { url: "http://runtime.test/v1/mcp-servers/mcp%2F1/enabled", method: "PUT", body: { repoRootId: "repo-1", enabled: null } },
    ]);
  });

  it("deletes an encoded server id and accepts no-content responses", async () => {
    const fetch = vi.fn(async () => new Response(null, { status: 204 }));
    const client = new AnyHarnessClient({ baseUrl: "http://runtime.test", fetch });

    await expect(client.mcpServers.remove("mcp/1")).resolves.toBeUndefined();
    const [url, options] = fetch.mock.calls[0] as unknown as [string, RequestInit];
    expect(url).toBe("http://runtime.test/v1/mcp-servers/mcp%2F1");
    expect(options.method).toBe("DELETE");
  });
});
