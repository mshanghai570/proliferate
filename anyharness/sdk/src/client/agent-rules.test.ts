import { expect, it, vi } from "vitest";
import { AnyHarnessClient } from "./core.js";
it("encodes rule scope, forwards cancellation, and preserves explicit empty and null writes", async () => {
  const fetch = vi.fn(async () => new Response(JSON.stringify({ agentKind: "custom", repositoryId: null, text: "", inherited: true })));
  const client = new AnyHarnessClient({ baseUrl: "http://runtime", authToken: "synthetic", fetch });
  const signal = new AbortController().signal;
  await client.agentRules.get("custom/name", "repo/a b", { signal });
  const [url, options] = fetch.mock.calls[0] as unknown as [string, RequestInit];
  expect(url).toBe("http://runtime/v1/agent-rules/custom%2Fname?repositoryId=repo%2Fa%20b");
  expect(options.signal).toBe(signal);
  expect(new Headers(options.headers).get("authorization")).toBe("Bearer synthetic");
  await client.agentRules.set("codex", { repositoryId: "repo", text: "" });
  await client.agentRules.set("codex", { repositoryId: "repo", text: null });
  expect(JSON.parse(String((fetch.mock.calls[1] as unknown as [string, RequestInit])[1].body))).toEqual({ repositoryId: "repo", text: "" });
  expect(JSON.parse(String((fetch.mock.calls[2] as unknown as [string, RequestInit])[1].body))).toEqual({ repositoryId: "repo", text: null });
});
