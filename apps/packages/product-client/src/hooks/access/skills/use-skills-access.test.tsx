// @vitest-environment jsdom
import { cleanup, renderHook } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { ReactNode } from "react";
import { useSkillsAccess } from "./use-skills-access";
import { useCustomAgents } from "#product/hooks/access/anyharness/agents/use-custom-agents";

const mocks = vi.hoisted(() => ({ resolve: vi.fn(() => { throw new Error("No runtime"); }) }));
vi.mock("@anyharness/sdk-react", () => ({
  useAnyHarnessRuntimeContext: () => ({ runtimeUrl: null }),
  useAnyHarnessCacheScopeKey: () => "actor",
  resolveRuntimeCacheScopeKey: () => "actor",
  resolveRuntimeConnection: mocks.resolve,
  getAnyHarnessClient: vi.fn(),
}));
vi.mock("#product/hooks/access/anyharness/agents/use-agent-resources-cache", () => ({
  useAgentResourcesCache: () => ({ invalidateAgentLaunchReadinessResources: vi.fn() }),
}));

describe("agent settings without a runtime", () => {
  afterEach(() => { cleanup(); vi.clearAllMocks(); });
  function wrapper({ children }: { children: ReactNode }) {
    return <QueryClientProvider client={new QueryClient({ defaultOptions: { queries: { retry: false } } })}>{children}</QueryClientProvider>;
  }
  it("renders disconnected skill settings without resolving an unavailable connection", () => {
    const { result } = renderHook(() => useSkillsAccess(), { wrapper });
    expect(result.current.available).toBe(false);
    expect(result.current.query.fetchStatus).toBe("idle");
    expect(mocks.resolve).not.toHaveBeenCalled();
  });
  it("renders disconnected ACP settings without resolving an unavailable connection", () => {
    const { result } = renderHook(() => useCustomAgents(true), { wrapper });
    expect(result.current.runtimeAvailable).toBe(false);
    expect(result.current.query.fetchStatus).toBe("idle");
    expect(mocks.resolve).not.toHaveBeenCalled();
  });
});
