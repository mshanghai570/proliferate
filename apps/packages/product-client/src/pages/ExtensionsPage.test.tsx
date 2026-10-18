// @vitest-environment jsdom
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { cleanup, render, screen } from "@testing-library/react";
import { afterEach, expect, it, vi } from "vitest";
import type { ReactNode } from "react";
import { ExtensionsPage } from "./ExtensionsPage";

vi.mock("@proliferate/product-client/host/ProductHostProvider", () => ({
  useProductHost: () => ({ desktop: {} }),
}));

vi.mock("#product/components/workspace/shell/screen/MainSidebarPageShell", () => ({
  MainSidebarPageShell: ({ children }: { children: ReactNode }) => <>{children}</>,
}));

afterEach(() => { cleanup(); vi.unstubAllEnvs(); vi.unstubAllGlobals(); });

it("does not contact the registry while discovery is disabled", () => {
  vi.stubEnv("VITE_EXTENSION_MARKETPLACE", "0");
  const fetchImpl = vi.fn();
  vi.stubGlobal("fetch", fetchImpl);
  render(<ExtensionsPage />);
  expect(screen.getByRole("heading", { name: "Extensions discovery is disabled" })).toBeTruthy();
  expect(fetchImpl).not.toHaveBeenCalled();
});

it("mounts discovery and requests the registry when explicitly enabled", async () => {
  vi.stubEnv("VITE_EXTENSION_MARKETPLACE", "1");
  const fetchImpl = vi.fn(async () => new Response(JSON.stringify({ extensions: [], totalSize: 0, offset: 0 })));
  vi.stubGlobal("fetch", fetchImpl);
  const client = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  render(<QueryClientProvider client={client}><ExtensionsPage /></QueryClientProvider>);
  await screen.findByText("No extensions found");
  expect(fetchImpl).toHaveBeenCalledOnce();
  client.clear();
});
