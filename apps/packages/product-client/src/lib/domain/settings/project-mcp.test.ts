import { describe, expect, it } from "vitest";
import { parseProjectMcpServer } from "./project-mcp";

describe("project MCP drafts", () => {
  it("maps HTTP headers and stdio environment into the runtime contract", () => {
    expect(parseProjectMcpServer("docs", JSON.stringify({ transport: "http", url: "http://localhost:8000/mcp", headers: { Authorization: "Bearer secret" } }))).toEqual({
      transport: "http", connectionId: "local-project:docs", serverName: "docs", catalogEntryId: null,
      url: "http://localhost:8000/mcp", headers: [{ name: "Authorization", value: "Bearer secret" }],
    });
    expect(parseProjectMcpServer("files", JSON.stringify({ transport: "stdio", command: "mcp-server", args: ["--project", "a b"], env: { API_KEY: "secret" } }))).toMatchObject({
      transport: "stdio", command: "mcp-server", args: ["--project", "a b"], env: [{ name: "API_KEY", value: "secret" }],
    });
  });

  it("rejects unknown fields, malformed values, reserved names, and oversized input", () => {
    for (const value of [null, [], { transport: "sse", url: "x" }, { transport: "http", url: "x", oauth: {} }, { transport: "stdio", command: "mcp", args: [3] }, { transport: "stdio", command: "mcp", args: null }, { transport: "stdio", command: "mcp", env: { API_KEY: 3 } }]) {
      expect(() => parseProjectMcpServer("docs", JSON.stringify(value))).toThrow();
    }
    for (const name of ["", "bad/name", "proliferate_workspace", "x".repeat(51)]) expect(() => parseProjectMcpServer(name, "{}")).toThrow();
    expect(() => parseProjectMcpServer("docs", " ".repeat(65_537))).toThrow(/64 KiB/);
  });

  it("does not echo credentials in parse errors", () => {
    try { parseProjectMcpServer("docs", '{"private-token":secret-token}'); } catch (error) {
      expect(String(error)).not.toContain("token");
      expect(String(error)).toContain("valid JSON");
    }
  });
});
