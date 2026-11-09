// @vitest-environment jsdom
import { AnyHarnessRuntime } from "@anyharness/sdk-react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { ProjectMcpSettings } from "./ProjectMcpSettings";

afterEach(cleanup);
function view(fetch: typeof globalThis.fetch, client: QueryClient, scope = "actor-a", repo = "repo-1") {
  return <QueryClientProvider client={client}><AnyHarnessRuntime runtimeUrl="http://runtime.test" cacheScopeKey={scope} fetch={fetch}><ProjectMcpSettings repoRootId={repo} /></AnyHarnessRuntime></QueryClientProvider>;
}
function client() { return new QueryClient({ defaultOptions: { queries: { retry: false } } }); }
function fill(name = "docs") {
  fireEvent.change(screen.getByLabelText("Server name"), { target: { value: name } });
  fireEvent.change(screen.getByLabelText("Server configuration (JSON)"), { target: { value: JSON.stringify({ transport: "http", url: "http://localhost:8000/mcp", headers: { Authorization: "Bearer draft-secret" } }) } });
}

describe("project MCP settings", () => {
  it("saves a real server request, clears secrets, toggles, and confirms removal", async () => {
    let rows: { name: string; transport: string; enabled: boolean }[] = [];
    const fetch = vi.fn(async (_url: RequestInfo | URL, init?: RequestInit) => {
      if (init?.method === "PUT") rows = [{ name: "docs", transport: "http", enabled: true }];
      if (init?.method === "PATCH") rows[0].enabled = false;
      if (init?.method === "DELETE") rows = [];
      if (init?.method === "DELETE") return new Response(null, { status: 204 });
      return init?.method === "GET" || !init?.method
        ? new Response(JSON.stringify({ servers: rows, encryptionAvailable: true })) : new Response(JSON.stringify(rows[0]));
    });
    const cache = client();
    render(view(fetch, cache));
    await screen.findByText("No project MCP servers configured.", {}, { timeout: 10_000 });
    fill();
    fireEvent.click(screen.getByRole("button", { name: "Save server" }));
    await screen.findByRole("button", { name: "Disable docs" });
    const request = fetch.mock.calls.find(([, init]) => init?.method === "PUT");
    expect(String(request?.[0])).toContain("/repo-roots/repo-1/mcp-servers/docs");
    expect(JSON.parse(String(request?.[1]?.body))).toMatchObject({ serverName: "docs", headers: [{ name: "Authorization", value: "Bearer draft-secret" }] });
    await waitFor(() => expect((screen.getByLabelText("Server configuration (JSON)") as HTMLTextAreaElement).value).toBe(""));
    expect(cache.getMutationCache().getAll()).toHaveLength(0);
    expect(JSON.stringify(cache.getQueryCache().getAll().map((query) => query.state.data))).not.toContain("draft-secret");
    fireEvent.click(screen.getByRole("button", { name: "Disable docs" }));
    await screen.findByRole("button", { name: "Enable docs" });
    fireEvent.click(screen.getByRole("button", { name: "Remove docs" }));
    expect(fetch.mock.calls.some(([, init]) => init?.method === "DELETE")).toBe(false);
    fireEvent.click(screen.getByRole("button", { name: "Remove server" }));
    await screen.findByText("No project MCP servers configured.");
    expect(fetch.mock.calls.some(([, init]) => init?.method === "DELETE")).toBe(true);
  });

  it("confirms replacement before submitting credentials", async () => {
    const fetch = vi.fn(async (_url: RequestInfo | URL, init?: RequestInit) => init?.method === "PUT"
      ? new Response(JSON.stringify({ name: "docs", transport: "http", enabled: true }))
      : new Response(JSON.stringify({ servers: [{ name: "docs", transport: "http", enabled: true }], encryptionAvailable: true })));
    render(view(fetch, client()));
    await screen.findByRole("button", { name: "Disable docs" });
    fill(); fireEvent.click(screen.getByRole("button", { name: "Save server" }));
    await screen.findByRole("button", { name: "Replace server" });
    expect(fetch.mock.calls.some(([, init]) => init?.method === "PUT")).toBe(false);
    fireEvent.click(screen.getByRole("button", { name: "Replace server" }));
    await waitFor(() => expect(fetch.mock.calls.some(([, init]) => init?.method === "PUT")).toBe(true));
  });

  it("clears a saved credential draft without waiting for the inventory refresh", async () => {
    let reads = 0;
    let finishRefresh: ((response: Response) => void) | undefined;
    const fetch = vi.fn(async (_url: RequestInfo | URL, init?: RequestInit) => {
      if (init?.method === "PUT") return new Response(JSON.stringify({ name: "docs", transport: "http", enabled: true }));
      if (++reads > 1) return new Promise<Response>((resolve) => { finishRefresh = resolve; });
      return new Response(JSON.stringify({ servers: [], encryptionAvailable: true }));
    });
    render(view(fetch, client()));
    await screen.findByText("No project MCP servers configured.");
    fill(); fireEvent.click(screen.getByRole("button", { name: "Save server" }));
    try {
      await waitFor(() => expect((screen.getByLabelText("Server configuration (JSON)") as HTMLTextAreaElement).value).toBe(""));
      expect(screen.getByRole("button", { name: "Disable docs" })).toBeTruthy();
    } finally { finishRefresh?.(new Response(JSON.stringify({ servers: [{ name: "docs", transport: "http", enabled: true }], encryptionAvailable: true }))); }
  });

  it("surfaces failed removal outside the confirmation dialog and keeps the server", async () => {
    const fetch = vi.fn(async (_url: RequestInfo | URL, init?: RequestInit) => init?.method === "DELETE"
      ? new Response(JSON.stringify({ title: "private-response-marker" }), { status: 503 })
      : new Response(JSON.stringify({ servers: [{ name: "docs", transport: "http", enabled: true }], encryptionAvailable: true })));
    render(view(fetch, client()));
    fireEvent.click(await screen.findByRole("button", { name: "Remove docs" }));
    fireEvent.click(screen.getByRole("button", { name: "Remove server" }));
    await screen.findByText(/Could not save MCP settings/);
    expect(screen.queryByRole("dialog")).toBeNull();
    expect(screen.getByRole("button", { name: "Remove docs" })).toBeTruthy();
    expect(screen.queryByText(/private-response-marker/)).toBeNull();
  });

  it("discards drafts when the actor or repository changes", async () => {
    const fetch = vi.fn(async () => new Response(JSON.stringify({ servers: [], encryptionAvailable: true })));
    const cache = client();
    const rendered = render(view(fetch, cache));
    await screen.findByText("No project MCP servers configured.");
    fill(); rendered.rerender(view(fetch, cache, "actor-b"));
    expect((screen.getByLabelText("Server configuration (JSON)") as HTMLTextAreaElement).value).toBe("");
    await screen.findByText("No project MCP servers configured.");
    fill(); rendered.rerender(view(fetch, cache, "actor-b", "repo-2"));
    expect((screen.getByLabelText("Server name") as HTMLInputElement).value).toBe("");
  });

  it("shows safe feedback for unavailable runtimes and missing encryption keys", async () => {
    const fetch = vi.fn(async () => new Response(JSON.stringify({ title: "private-response-marker" }), { status: 404 }));
    const rendered = render(view(fetch, client()));
    await screen.findByText("Could not load project MCP settings.");
    expect(screen.queryByText(/private-response-marker/)).toBeNull();
    expect((screen.getByRole("button", { name: "Save server" }) as HTMLButtonElement).disabled).toBe(true);
    rendered.unmount();
    render(view(vi.fn(async () => new Response(JSON.stringify({ servers: [], encryptionAvailable: false }))), client()));
    await screen.findByText(/runtime encryption key is unavailable/);
    expect((screen.getByRole("button", { name: "Save server" }) as HTMLButtonElement).disabled).toBe(true);
  });
});
