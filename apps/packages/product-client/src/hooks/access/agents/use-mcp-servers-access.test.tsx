// @vitest-environment jsdom
import { cleanup, renderHook, waitFor } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { ReactNode } from "react";
import { useMcpServersAccess } from "./use-mcp-servers-access";
import { createMcpServer, listMcpServers } from "#product/lib/access/anyharness/mcp-servers";

const mocks = vi.hoisted(() => ({
  available: false,
  resolve: vi.fn(() => {
    if (!mocks.available) throw new Error("No runtime");
    return { baseUrl: "http://runtime.test" };
  }),
}));

vi.mock("@anyharness/sdk-react", () => ({
  useAnyHarnessRuntimeContext: () => ({ runtimeUrl: mocks.available ? "http://runtime.test" : null }),
  resolveRuntimeCacheScopeKey: () => "test-runtime",
  resolveRuntimeConnection: mocks.resolve,
}));

vi.mock("#product/lib/access/anyharness/mcp-servers", () => ({
  createMcpServer: vi.fn(async () => ({})),
  listMcpServers: vi.fn(async () => ({ servers: [] })),
  removeMcpServer: vi.fn(async () => undefined),
  setMcpServerEnabled: vi.fn(async () => ({})),
}));

describe("useMcpServersAccess", () => {
  afterEach(() => {
    cleanup();
    vi.clearAllMocks();
    mocks.available = false;
  });

  function makeWrapper() {
    const client = new QueryClient({
      defaultOptions: { queries: { retry: false }, mutations: { retry: false } },
    });
    return {
      client,
      wrapper: ({ children }: { children: ReactNode }) => (
        <QueryClientProvider client={client}>{children}</QueryClientProvider>
      ),
    };
  }

  it("stays idle and does not resolve a connection while disconnected", () => {
    const { wrapper } = makeWrapper();
    const { result } = renderHook(() => useMcpServersAccess(), { wrapper });

    expect(result.current.available).toBe(false);
    expect(result.current.query.fetchStatus).toBe("idle");
    expect(mocks.resolve).not.toHaveBeenCalled();
  });

  it("lists through the runtime connection and invalidates after writes", async () => {
    mocks.available = true;
    const { wrapper } = makeWrapper();
    const { result } = renderHook(() => useMcpServersAccess(), { wrapper });

    await waitFor(() => expect(listMcpServers).toHaveBeenCalledTimes(1));
    await result.current.mutation.mutateAsync({
      kind: "create",
      request: {
        serverName: "filesystem",
        config: { transport: "stdio", command: "mcp-filesystem" },
      },
    });
    await waitFor(() => expect(listMcpServers).toHaveBeenCalledTimes(2));
    expect(result.current.available).toBe(true);
    expect(createMcpServer).toHaveBeenCalledTimes(1);
  });
});
