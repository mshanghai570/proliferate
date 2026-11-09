// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { McpServerSummary } from "@anyharness/sdk";
import { McpServersPane } from "./McpServersPane";
import { MCP_SERVERS_COPY as copy } from "#product/copy/settings/agents/mcp-servers-copy";

const state = vi.hoisted(() => ({
  local: true,
  access: null as any,
  repositories: [{ id: "repo-1", path: "/workspace/repo-one" }],
}));

vi.mock("@anyharness/sdk-react", () => ({
  useAnyHarnessRuntimeContext: () => ({ runtimeUrl: "http://runtime.test" }),
  resolveRuntimeCacheScopeKey: () => "runtime",
  useRepoRootsQuery: () => ({ data: state.repositories, isError: false }),
}));

vi.mock("#product/hooks/access/agents/use-mcp-servers-access", () => ({
  useMcpServersAccess: () => state.access,
}));

vi.mock("#product/stores/ui/agent-surface-store", () => ({
  useAgentSurfaceStore: (selector: (value: { surface: string }) => unknown) =>
    selector({ surface: state.local ? "local" : "cloud" }),
}));

const stdioServer: McpServerSummary = {
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

function setup(servers: McpServerSummary[] = []) {
  const access = {
    available: true,
    query: {
      data: { servers },
      isPending: false,
      isError: false,
      isSuccess: true,
      refetch: vi.fn(),
    },
    mutation: {
      isPending: false,
      isError: false,
      error: null,
      mutate: vi.fn(),
      mutateAsync: vi.fn(async () => undefined),
    },
  };
  state.local = true;
  state.access = access;
  const view = render(<McpServersPane />);
  return { ...view, access };
}

describe("McpServersPane", () => {
  beforeEach(() => {
    state.local = true;
    state.repositories = [{ id: "repo-1", path: "/workspace/repo-one" }];
  });
  afterEach(() => cleanup());

  it("creates a stdio server with arguments and a one-time masked secret", async () => {
    const { access, container } = setup();
    fireEvent.change(screen.getByLabelText(copy.serverName), { target: { value: "filesystem" } });
    fireEvent.change(screen.getByLabelText(copy.command), { target: { value: "mcp-filesystem" } });
    fireEvent.change(screen.getByLabelText(copy.args), { target: { value: "/workspace\n--read-only" } });
    fireEvent.change(screen.getByLabelText(copy.credentialName), { target: { value: "FILESYSTEM_TOKEN" } });
    const secret = "never-render-this-secret";
    fireEvent.change(screen.getByLabelText(copy.credentialValue), { target: { value: secret } });

    expect(screen.getByLabelText(copy.credentialValue).getAttribute("type")).toBe("password");
    expect(container.textContent).not.toContain(secret);
    fireEvent.click(screen.getByRole("button", { name: copy.addServer }));
    await waitFor(() => expect(access.mutation.mutateAsync).toHaveBeenCalledWith({
      kind: "create",
      request: {
        serverName: "filesystem",
        config: {
          transport: "stdio",
          command: "mcp-filesystem",
          args: ["/workspace", "--read-only"],
          env: [{ name: "FILESYSTEM_TOKEN", value: secret }],
        },
      },
    }));
    await waitFor(() => expect((screen.getByLabelText(copy.credentialValue) as HTMLInputElement).value).toBe(""));
  });

  it("preserves empty and whitespace-only command arguments", async () => {
    const { access } = setup();
    fireEvent.change(screen.getByLabelText(copy.serverName), { target: { value: "stdio" } });
    fireEvent.change(screen.getByLabelText(copy.command), { target: { value: "mcp-server" } });
    fireEvent.change(screen.getByLabelText(copy.args), { target: { value: "first\n\n  \nlast" } });

    fireEvent.click(screen.getByRole("button", { name: copy.addServer }));
    await waitFor(() => expect(access.mutation.mutateAsync).toHaveBeenCalledWith({
      kind: "create",
      request: {
        serverName: "stdio",
        config: {
          transport: "stdio",
          command: "mcp-server",
          args: ["first", "", "  ", "last"],
          env: [],
        },
      },
    }));
  });

  it("creates an HTTP server with a header secret without rendering response secrets", async () => {
    const response = {
      ...stdioServer,
      serverName: "remote",
      transport: "http" as const,
      headerNames: ["Authorization"],
    };
    const { access, container } = setup([response]);
    fireEvent.change(screen.getByLabelText(copy.serverName), { target: { value: "remote" } });
    fireEvent.change(screen.getByLabelText(copy.transport), { target: { value: "http" } });
    fireEvent.change(screen.getByLabelText(copy.url), { target: { value: "https://mcp.example.test/api" } });
    fireEvent.change(screen.getByLabelText(copy.credentialName), { target: { value: "Authorization" } });
    const secret = "header-secret-value";
    fireEvent.change(screen.getByLabelText(copy.credentialValue), { target: { value: secret } });

    fireEvent.click(screen.getByRole("button", { name: copy.addServer }));
    await waitFor(() => expect(access.mutation.mutateAsync).toHaveBeenCalledWith({
      kind: "create",
      request: {
        serverName: "remote",
        config: {
          transport: "http",
          url: "https://mcp.example.test/api",
          headers: [{ name: "Authorization", value: secret }],
        },
      },
    }));
    expect(container.textContent).not.toContain(secret);
    expect(container.textContent).not.toContain("FILESYSTEM_TOKEN");
    expect(container.textContent).toContain("remote");
    expect(container.textContent).toContain(copy.http);
  });

  it("explains that a runtime data key is needed when secret storage is unavailable", async () => {
    const { access } = setup();
    access.mutation.mutateAsync.mockRejectedValueOnce({
      problem: { code: "MCP_SERVER_DATA_KEY_REQUIRED" },
    });
    fireEvent.change(screen.getByLabelText(copy.serverName), { target: { value: "filesystem" } });
    fireEvent.change(screen.getByLabelText(copy.command), { target: { value: "mcp-filesystem" } });
    fireEvent.click(screen.getByRole("button", { name: copy.addServer }));
    expect(await screen.findByText(copy.unavailableKey)).toBeTruthy();
  });

  it("sets a user default, a repository override, and resets inheritance", () => {
    const { access, rerender } = setup([{
      ...stdioServer,
      userEnabled: true,
      repositoryOverrides: { "repo-1": false },
    }]);
    fireEvent.click(screen.getByRole("switch", { name: `${copy.enabled} filesystem` }));
    expect(access.mutation.mutate).toHaveBeenLastCalledWith({
      kind: "enable",
      id: "mcp-1",
      request: false,
    });

    fireEvent.change(screen.getByLabelText(copy.scope), { target: { value: "repo-1" } });
    fireEvent.click(screen.getByRole("switch", { name: `${copy.enabled} filesystem` }));
    expect(access.mutation.mutate).toHaveBeenLastCalledWith({
      kind: "enable",
      id: "mcp-1",
      request: { repoRootId: "repo-1", enabled: true },
    });
    fireEvent.click(screen.getByRole("button", { name: copy.useDefault }));
    expect(access.mutation.mutate).toHaveBeenLastCalledWith({
      kind: "enable",
      id: "mcp-1",
      request: { repoRootId: "repo-1", enabled: null },
    });
    access.query.isError = true;
    access.mutation.isError = true;
    rerender(<McpServersPane />);
    expect(screen.getByText(copy.loadError)).toBeTruthy();
    expect(screen.getByText(copy.mutationError)).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: copy.retry }));
    expect(access.query.refetch).toHaveBeenCalledTimes(1);
  });

  it("requires confirmation before deletion", () => {
    const { access } = setup([stdioServer]);
    fireEvent.click(screen.getByRole("button", { name: copy.remove }));
    expect(access.mutation.mutate).not.toHaveBeenCalled();
    fireEvent.click(screen.getByRole("button", { name: copy.confirmRemove }));
    expect(access.mutation.mutate).toHaveBeenCalledWith(
      { kind: "remove", id: "mcp-1" },
      expect.objectContaining({ onSuccess: expect.any(Function) }),
    );

  });

  it("shows a disconnected state and the local-runtime gate", () => {
    const { rerender } = setup();
    state.access.available = false;
    rerender(<McpServersPane />);
    expect(screen.getByText(copy.disconnected)).toBeTruthy();

    state.local = false;
    rerender(<McpServersPane />);
    expect(screen.getByText(copy.localOnly)).toBeTruthy();
  });
});
