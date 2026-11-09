import { describe, expect, it, vi } from "vitest";
import { AnyHarnessClient } from "./core.js";

describe("extension management client", () => {
  it("lists registered state with authentication and cancellation", async () => {
    const response = { extensions: [{ namespace: "example", name: "theme", version: "1.0.0", sha256: "a".repeat(64), state: "installed", active: false, errorCode: null }] };
    const fetch = vi.fn(async () => new Response(JSON.stringify(response)));
    const client = new AnyHarnessClient({ baseUrl: "http://runtime.test", authToken: "synthetic-test-token", fetch });
    const signal = new AbortController().signal;
    expect(await client.extensions.list({ signal })).toEqual(response);
    const [url, options] = fetch.mock.calls[0] as unknown as [string, RequestInit];
    expect(url).toBe("http://runtime.test/v1/extensions");
    expect(new Headers(options.headers).get("authorization")).toBe("Bearer synthetic-test-token");
    expect(options.signal).toBe(signal);
  });

  it("posts a registry identity and returns the verified installed summary", async () => {
    const summary = { namespace: "example", name: "theme", version: "1.0.0", sha256: "a".repeat(64), state: "installed", active: false, errorCode: null };
    const fetch = vi.fn(async () => new Response(JSON.stringify(summary), { status: 201 }));
    const client = new AnyHarnessClient({ baseUrl: "http://runtime.test", authToken: "synthetic-test-token", fetch });
    expect(await client.extensions.install({ namespace: "example", name: "theme", version: "1.0.0" })).toEqual(summary);
    const [url, options] = fetch.mock.calls[0] as unknown as [string, RequestInit];
    expect(url).toBe("http://runtime.test/v1/extensions/install");
    expect(options.method).toBe("POST");
    expect(JSON.parse(String(options.body))).toEqual({ namespace: "example", name: "theme", version: "1.0.0" });
    expect(new Headers(options.headers).get("authorization")).toBe("Bearer synthetic-test-token");
  });

  it("encodes the selected identity and accepts an empty uninstall response", async () => {
    const fetch = vi.fn(async () => new Response(null, { status: 204 }));
    const client = new AnyHarnessClient({ baseUrl: "http://runtime.test", fetch });
    await client.extensions.uninstall({ namespace: "scope/name", name: "some theme", version: "1.0.0+build" });
    const [url, options] = fetch.mock.calls[0] as unknown as [string, RequestInit];
    expect(url).toBe("http://runtime.test/v1/extensions/scope%2Fname/some%20theme/1.0.0%2Bbuild");
    expect(options.method).toBe("DELETE");
  });
});
