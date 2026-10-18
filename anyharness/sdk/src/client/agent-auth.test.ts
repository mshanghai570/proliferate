import { describe, expect, it } from "vitest";

import type { AnyHarnessTransport } from "./core.js";
import { AgentAuthClient } from "./agent-auth.js";

describe("AgentAuthClient.clearState", () => {
  it("deletes the persisted route state", async () => {
    const calls: string[] = [];
    const transport = {
      delete: async (path: string) => {
        calls.push(path);
      },
    } as unknown as AnyHarnessTransport;
    const client = new AgentAuthClient(transport);

    await client.clearState();

    expect(calls).toEqual(["/v1/agent-auth/state"]);
  });
});

describe("runtime-local provider management", () => {
  it("uses the owner runtime transport and encodes provider identity", async () => {
    const calls: unknown[] = [];
    const transport = {
      get: async (...args: unknown[]) => { calls.push(["GET", ...args]); return { providers: [] }; },
      post: async (...args: unknown[]) => { calls.push(["POST", ...args]); return { id: "one" }; },
      patch: async (...args: unknown[]) => { calls.push(["PATCH", ...args]); return { id: "one" }; },
      delete: async (...args: unknown[]) => { calls.push(["DELETE", ...args]); },
    } as unknown as AnyHarnessTransport;
    const client = new AgentAuthClient(transport);
    const options = { signal: new AbortController().signal };
    const input = { name: "Local", endpoint: "http://localhost:1234/v1", models: ["model-a"], apiFormat: "chat", apiKey: null };
    await client.listLocalProviders(options);
    await client.createLocalProvider(input);
    await client.setLocalProviderEnabled("one/two", false);
    await client.removeLocalProvider("one/two");
    expect(calls).toEqual([
      ["GET", "/v1/agent-auth/local-providers", options],
      ["POST", "/v1/agent-auth/local-providers", input],
      ["PATCH", "/v1/agent-auth/local-providers/one%2Ftwo", { enabled: false }],
      ["DELETE", "/v1/agent-auth/local-providers/one%2Ftwo"],
    ]);
  });
});
