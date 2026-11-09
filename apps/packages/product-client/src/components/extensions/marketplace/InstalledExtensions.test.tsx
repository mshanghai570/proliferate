// @vitest-environment jsdom
import { AnyHarnessRuntime } from "@anyharness/sdk-react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { InstalledExtensions } from "./InstalledExtensions";

const extension = { namespace: "example", name: "theme", version: "1.0.0", sha256: "a".repeat(64), state: "installed", active: false, errorCode: null };
afterEach(cleanup);

function view(fetch: typeof globalThis.fetch, runtimeUrl: string | null = "http://runtime.test", scope = "actor-a", client = new QueryClient({ defaultOptions: { queries: { retry: false }, mutations: { retry: false } } })) {
  return <QueryClientProvider client={client}><AnyHarnessRuntime runtimeUrl={runtimeUrl} cacheScopeKey={scope} fetch={fetch}><InstalledExtensions /></AnyHarnessRuntime></QueryClientProvider>;
}

describe("installed extension management", () => {
  it("lists inactive packages and only deletes after confirmation, then refreshes", async () => {
    let installed = true;
    const fetch = vi.fn(async (_url: RequestInfo | URL, options?: RequestInit) => {
      if (options?.method === "DELETE") { installed = false; return new Response(null, { status: 204 }); }
      return new Response(JSON.stringify({ extensions: installed ? [extension] : [] }));
    });
    render(view(fetch));
    await screen.findByText("example.theme");
    expect(screen.getByText("Installed · Inactive")).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: "Uninstall example.theme@1.0.0" }));
    expect(fetch.mock.calls.some(([, options]) => options?.method === "DELETE")).toBe(false);
    fireEvent.click(screen.getByRole("button", { name: "Cancel" }));
    expect(document.activeElement).toBe(screen.getByRole("button", { name: "Uninstall example.theme@1.0.0" }));
    expect(fetch.mock.calls.some(([, options]) => options?.method === "DELETE")).toBe(false);
    fireEvent.click(screen.getByRole("button", { name: "Uninstall example.theme@1.0.0" }));
    fireEvent.click(screen.getByRole("button", { name: "Confirm uninstall" }));
    await screen.findByText("No installed extensions");
    expect(document.activeElement).toBe(screen.getByRole("button", { name: "Refresh" }));
    expect(fetch.mock.calls.filter(([, options]) => options?.method === "DELETE")).toHaveLength(1);
  });

  it("shows bounded package errors and does not remove a failed uninstall from the list", async () => {
    const fetch = vi.fn(async (_url: RequestInfo | URL, options?: RequestInit) => options?.method === "DELETE"
      ? new Response(JSON.stringify({ title: "private-error-marker", status: 409, code: "EXTENSION_PACKAGE_INVALID" }), { status: 409 })
      : new Response(JSON.stringify({ extensions: [{ ...extension, state: "error", errorCode: "package_invalid" }] })));
    render(view(fetch));
    await screen.findByText("Package validation failed");
    fireEvent.click(screen.getByRole("button", { name: "Uninstall example.theme@1.0.0" }));
    fireEvent.click(screen.getByRole("button", { name: "Confirm uninstall" }));
    await screen.findByRole("alert");
    expect(screen.queryByText(/private-error-marker/)).toBeNull();
    expect(screen.getByText("example.theme")).toBeTruthy();
  });

  it("does not request a runtime when disconnected", async () => {
    const fetch = vi.fn();
    render(view(fetch, null));
    expect(screen.getByText("Connect a runtime to manage installed extensions.")).toBeTruthy();
    expect(fetch).not.toHaveBeenCalled();
  });

  it("keeps installation state separate across runtime changes", async () => {
    const fetch = vi.fn(async (url: RequestInfo | URL) => new Response(JSON.stringify({ extensions: String(url).startsWith("http://runtime.test") ? [extension] : [] })));
    const client = new QueryClient({ defaultOptions: { queries: { retry: false } } });
    const rendered = render(view(fetch, "http://runtime.test", "actor-a", client));
    await screen.findByText("example.theme");
    fireEvent.click(screen.getByRole("button", { name: "Uninstall example.theme@1.0.0" }));
    rendered.rerender(view(fetch, "http://other.test", "actor-a", client));
    await screen.findByText("No installed extensions");
    expect(screen.queryByRole("button", { name: "Confirm uninstall" })).toBeNull();
    await waitFor(() => expect(fetch.mock.calls).toHaveLength(2));
  });

  it("does not reuse another actor's installed state on the same runtime", async () => {
    let calls = 0;
    const fetch = vi.fn(async () => new Response(JSON.stringify({ extensions: ++calls === 1 ? [extension] : [] })));
    const client = new QueryClient({ defaultOptions: { queries: { retry: false } } });
    const rendered = render(view(fetch, "http://runtime.test", "actor-a", client));
    await screen.findByText("example.theme");
    rendered.rerender(view(fetch, "http://runtime.test", "actor-b", client));
    await screen.findByText("No installed extensions");
    expect(fetch).toHaveBeenCalledTimes(2);
  });

  it("reports older runtimes without manufacturing an empty installed list", async () => {
    const fetch = vi.fn(async () => new Response(JSON.stringify({ title: "Not found", status: 404 }), { status: 404 }));
    render(view(fetch));
    await screen.findByText("This runtime does not support extension management. Update the runtime and retry.");
    expect(screen.queryByText("No installed extensions")).toBeNull();
    expect(screen.getByRole("button", { name: "Retry" })).toBeTruthy();
  });
});
