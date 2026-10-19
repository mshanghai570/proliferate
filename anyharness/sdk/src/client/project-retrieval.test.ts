import { expect, it, vi } from "vitest";
import { AnyHarnessClient } from "./core.js";
it("scopes all retrieval operations and forwards auth and cancellation", async () => {
  const fetch = vi.fn(async (_url: string, options?: RequestInit) => options?.method === "DELETE" ? new Response(null, { status: 204 }) : new Response(JSON.stringify({ enabled: false })));
  const client = new AnyHarnessClient({ baseUrl: "http://runtime", authToken: "synthetic", fetch: fetch as typeof globalThis.fetch });
  const signal = new AbortController().signal;
  await client.projectRetrieval.status("work/a b", { signal });
  await client.projectRetrieval.index("work/a b");
  await client.projectRetrieval.search("work/a b", { query: "quoted source" });
  await client.projectRetrieval.clear("work/a b");
  expect(fetch.mock.calls.map(([url]) => url)).toEqual([
    "http://runtime/v1/workspaces/work%2Fa%20b/retrieval",
    "http://runtime/v1/workspaces/work%2Fa%20b/retrieval",
    "http://runtime/v1/workspaces/work%2Fa%20b/retrieval/search",
    "http://runtime/v1/workspaces/work%2Fa%20b/retrieval",
  ]);
  expect(fetch.mock.calls[0]?.[1]?.signal).toBe(signal);
  expect(new Headers(fetch.mock.calls[0]?.[1]?.headers).get("authorization")).toBe("Bearer synthetic");
  expect(JSON.parse(String(fetch.mock.calls[2]?.[1]?.body))).toEqual({ query: "quoted source" });
  expect(fetch.mock.calls[3]?.[1]?.method).toBe("DELETE");
});
