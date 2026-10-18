import { describe, expect, it, vi } from "vitest";

import type { ReadWorkspaceFileResponse } from "../types/files.js";
import type {
  MaterializeRepoRootRequest,
  MaterializeWorkspaceAtRefRequest,
} from "../types/repo-roots.js";
import { AnyHarnessClient, type AnyHarnessTransport } from "./core.js";
import { RepoRootsClient } from "./repo-roots.js";

describe("project MCP management", () => {
  it("lists with authentication, cancellation, and an encoded project ID", async () => {
    const inventory = { servers: [], encryptionAvailable: true };
    const fetch = vi.fn(async () => new Response(JSON.stringify(inventory)));
    const client = new AnyHarnessClient({ baseUrl: "http://runtime.test", authToken: "synthetic-token", fetch });
    const signal = new AbortController().signal;
    expect(await client.repoRoots.listMcpServers("repo/root", { signal })).toEqual(inventory);
    const [url, options] = fetch.mock.calls[0] as unknown as [string, RequestInit];
    expect(url).toBe("http://runtime.test/v1/repo-roots/repo%2Froot/mcp-servers");
    expect(options.signal).toBe(signal);
    expect(new Headers(options.headers).get("authorization")).toBe("Bearer synthetic-token");
  });

  it("accepts safe JSON save/toggle responses and empty removal responses", async () => {
    const summary = { name: "docs", transport: "http", enabled: true };
    const fetch = vi.fn(async (_url: RequestInfo | URL, options?: RequestInit) => options?.method === "DELETE"
      ? new Response(null, { status: 204 }) : new Response(JSON.stringify(summary)));
    const client = new AnyHarnessClient({ baseUrl: "http://runtime.test", fetch });
    const server = { transport: "http" as const, connectionId: "ignored", serverName: "docs", url: "http://localhost:8000/mcp" };
    expect(await client.repoRoots.putMcpServer("repo/root", "docs tools", server)).toEqual(summary);
    expect(await client.repoRoots.setMcpServerEnabled("repo/root", "docs tools", false)).toEqual(summary);
    await client.repoRoots.removeMcpServer("repo/root", "docs tools");
    expect(fetch.mock.calls.map(([url, options]) => [String(url), options?.method])).toEqual([
      ["http://runtime.test/v1/repo-roots/repo%2Froot/mcp-servers/docs%20tools", "PUT"],
      ["http://runtime.test/v1/repo-roots/repo%2Froot/mcp-servers/docs%20tools", "PATCH"],
      ["http://runtime.test/v1/repo-roots/repo%2Froot/mcp-servers/docs%20tools", "DELETE"],
    ]);
    expect(JSON.parse(String(fetch.mock.calls[0][1]?.body))).toEqual(server);
    expect(JSON.parse(String(fetch.mock.calls[1][1]?.body))).toEqual({ enabled: false });
  });
});

describe("RepoRootsClient.readFile", () => {

  it("URL-encodes the repo root id and relative path", async () => {
    const calls: string[] = [];
    const response: ReadWorkspaceFileResponse = {
      path: "dir/file name.ts",
      kind: "file",
      content: "contents",
      versionToken: "token",
      encoding: "utf-8",
      sizeBytes: 8,
      isText: true,
      tooLarge: false,
    };
    const transport = {
      get: async (path: string) => {
        calls.push(path);
        return response;
      },
    } as unknown as AnyHarnessTransport;
    const client = new RepoRootsClient(transport);

    await client.readFile("repo/root", "dir/file name.ts");

    expect(calls).toEqual([
      "/v1/repo-roots/repo%2Froot/files/file?path=dir%2Ffile%20name.ts",
    ]);
  });
});

describe("RepoRootsClient materialization", () => {
  it("posts repo-root acquisition to the materializations endpoint", async () => {
    const calls: Array<{ path: string; body: unknown }> = [];
    const transport = {
      post: async (path: string, body: unknown) => {
        calls.push({ path, body });
        return {};
      },
    } as unknown as AnyHarnessTransport;
    const client = new RepoRootsClient(transport);

    const input: MaterializeRepoRootRequest = {
      operationId: "op-1",
      repository: {
        provider: "github",
        owner: "acme",
        name: "widget",
        cloneUrl: "https://github.com/acme/widget.git",
      },
      destinationPath: "/tmp/widget",
      mode: "clone_or_adopt",
    };
    await client.materialize(input);

    expect(calls).toEqual([
      { path: "/v1/repo-roots/materializations", body: input },
    ]);
  });

  it("URL-encodes the repo root id for workspace materialization", async () => {
    const calls: Array<{ path: string; body: unknown }> = [];
    const transport = {
      post: async (path: string, body: unknown) => {
        calls.push({ path, body });
        return {};
      },
    } as unknown as AnyHarnessTransport;
    const client = new RepoRootsClient(transport);

    const input: MaterializeWorkspaceAtRefRequest = {
      operationId: "op-2",
      branchName: "feature/x",
      headSha: "deadbeef",
    };
    await client.materializeWorkspaceAtRef("repo/root", input);

    expect(calls).toEqual([
      {
        path: "/v1/repo-roots/repo%2Froot/workspace-materializations",
        body: input,
      },
    ]);
  });
});
