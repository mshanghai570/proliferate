// @vitest-environment jsdom
import { AnyHarnessRuntime } from "@anyharness/sdk-react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { ExtensionRegistryProvider } from "#product/providers/ExtensionRegistryProvider";
import { ExtensionMarketplaceScreen } from "./ExtensionMarketplaceScreen";

const extension = { namespace: "example", name: "theme", displayName: "Example Theme", version: "1.0.0", description: "Quiet syntax colors", downloadCount: 42 };
const requests: string[] = [];
function renderMarketplace(failSearch = false, runtimeUrl: string | null = null, runtimeFetch?: typeof globalThis.fetch) {
  vi.stubGlobal("fetch", vi.fn(async (input: string) => {
    requests.push(input);
    const url = new URL(input);
    if (url.pathname.endsWith("/search") && failSearch) return new Response("", { status: 503 });
    if (url.pathname.endsWith("/search")) return new Response(JSON.stringify({
      extensions: [extension], totalSize: 25, offset: Number(url.searchParams.get("offset") ?? 0),
    }));
    if (url.pathname.endsWith("/manifest")) return new Response(JSON.stringify({
      name: "theme", version: "1.0.0", engines: { vscode: "^1.60.0" }, contributes: { themes: [{ path: "theme.json" }] },
    }));
    if (url.pathname.endsWith("/readme")) return new Response('# Theme guide\n\n[Unsafe](command:run)\n\n<script>window.stolen=true</script>');
    return new Response(JSON.stringify({ ...extension,
      files: { readme: "https://open-vsx.org/readme", manifest: "https://open-vsx.org/manifest" },
      allVersions: { "1.0.0": "https://open-vsx.org/api/example/theme/1.0.0" },
      dependencies: ["example.base"],
    }));
  }));
  const client = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  render(<QueryClientProvider client={client}><AnyHarnessRuntime runtimeUrl={runtimeUrl} fetch={runtimeFetch}><ExtensionRegistryProvider><ExtensionMarketplaceScreen /></ExtensionRegistryProvider></AnyHarnessRuntime></QueryClientProvider>);
  return client;
}

afterEach(() => { cleanup(); vi.unstubAllGlobals(); requests.length = 0; });

describe("extension discovery", () => {
  it("loads search, category, sorting, and another page from the registry", async () => {
    renderMarketplace();
    await screen.findByRole("button", { name: /Example Theme/ }, { timeout: 5000 });
    fireEvent.change(screen.getByRole("searchbox", { name: "Search extensions" }), { target: { value: "syntax" } });
    await waitFor(() => expect(requests.some((url) => new URL(url).searchParams.get("query") === "syntax")).toBe(true));
    fireEvent.change(screen.getByLabelText("Category"), { target: { value: "Themes" } });
    await waitFor(() => expect(requests.some((url) => new URL(url).searchParams.get("category") === "Themes")).toBe(true));
    fireEvent.change(screen.getByLabelText("Sort by"), { target: { value: "rating" } });
    await waitFor(() => expect(requests.some((url) => new URL(url).searchParams.get("sortBy") === "rating")).toBe(true));
    await waitFor(() => expect((screen.getByRole("button", { name: "Next page" }) as HTMLButtonElement).disabled).toBe(false));
    fireEvent.click(screen.getByRole("button", { name: "Next page" }));
    await waitFor(() => expect(requests.some((url) => new URL(url).searchParams.get("offset") === "24")).toBe(true));
  });

  it("opens metadata and sanitized README, reports unsupported capabilities, and keeps installation unavailable without a runtime", async () => {
    renderMarketplace();
    fireEvent.click(await screen.findByRole("button", { name: /Example Theme/ }, { timeout: 5000 }));
    await screen.findByRole("heading", { name: "Theme guide" });
    expect(document.querySelector("script")).toBeNull();
    expect(screen.getByText("Unsafe").getAttribute("href")).toBeNull();
    expect((screen.getByRole("button", { name: "Install" }) as HTMLButtonElement).disabled).toBe(true);
    expect(screen.getByText("example.base")).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: "Capabilities" }));
    await screen.findByText("themes");
    expect(screen.getByText(/No extension host/)).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: "Back to extensions" }));
    expect(document.activeElement).toBe(screen.getByRole("searchbox", { name: "Search extensions" }));
  });

  it("installs through the connected runtime and announces the inactive result", async () => {
    const runtimeFetch = vi.fn(async () => new Response(JSON.stringify({
      namespace: "example", name: "theme", version: "1.0.0", active: false,
      state: "installed", sha256: "a".repeat(64), errorCode: null,
    }), { status: 201 }));
    renderMarketplace(false, "http://runtime.test", runtimeFetch);
    fireEvent.click(await screen.findByRole("button", { name: /Example Theme/ }, { timeout: 5000 }));
    await screen.findByRole("heading", { name: "Theme guide" });
    fireEvent.click(screen.getByRole("button", { name: "Install" }));
    expect(await screen.findByText("Extension installed. On macOS, check Installed for supported app color themes. Extension code remains inactive.")).toBeTruthy();
    await waitFor(() => expect(runtimeFetch).toHaveBeenCalledWith("http://runtime.test/v1/extensions/install", expect.objectContaining({ method: "POST" })));
    expect(runtimeFetch.mock.calls[0]?.[1]).toMatchObject({
      body: JSON.stringify({ namespace: "example", name: "theme", version: "1.0.0" }),
    });
  });

  it("opens registry details from Installed and returns with keyboard focus", async () => {
    const runtimeFetch = vi.fn(async (_url: RequestInfo | URL, _options?: RequestInit) => new Response(JSON.stringify({ extensions: [{
      namespace: "example", name: "theme", version: "0.9.0", active: false,
      state: "installed", sha256: "a".repeat(64), errorCode: null,
    }] })));
    renderMarketplace(false, "http://runtime.test", runtimeFetch);
    fireEvent.click(screen.getByRole("button", { name: "Installed" }));
    fireEvent.click(await screen.findByRole("button", { name: "Details and versions for example.theme" }));
    await screen.findByRole("heading", { name: "Theme guide" });
    fireEvent.click(screen.getByRole("button", { name: "Versions" }));
    await screen.findByRole("heading", { name: "Version history" });
    expect(screen.getByText("1.0.0")).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: "Back to extensions" }));
    await screen.findByText("0.9.0");
    const installed = screen.getByRole("button", { name: "Installed" });
    expect(installed.getAttribute("aria-pressed")).toBe("true");
    expect(document.activeElement).toBe(installed);
    expect(runtimeFetch.mock.calls.every(([, options]) => !options?.method || options.method === "GET")).toBe(true);
  });

  it("shows an actionable failure and retries", async () => {
    renderMarketplace(true);
    await screen.findByRole("button", { name: "Retry" });
    vi.mocked(fetch).mockResolvedValue(new Response(JSON.stringify({ extensions: [], totalSize: 0, offset: 0 })));
    fireEvent.click(screen.getByRole("button", { name: "Retry" }));
    await screen.findByText("No extensions found");
  });
});
