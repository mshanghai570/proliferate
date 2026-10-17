import { describe, expect, it, vi } from "vitest";
import { fetchExtensionContent } from "./extension-content";
import { resolveExtensionContentUrl } from "#product/lib/domain/extensions/extension-content";

describe("untrusted extension content", () => {
  it("resolves relative images and permits only HTTPS without credentials", () => {
    expect(resolveExtensionContentUrl("./assets/demo.png", "https://open-vsx.org/api/a/b/readme.md"))
      .toBe("https://open-vsx.org/api/a/b/assets/demo.png");
    for (const value of ["javascript:alert(1)", "command:run", "file:///private/key", "data:image/svg+xml,x", "http://a.test", "https://user:pass@a.test"]) {
      expect(resolveExtensionContentUrl(value)).toBeNull();
    }
  });

  it("requests public content without cookies and passes cancellation", async () => {
    const controller = new AbortController();
    const fetchImpl = vi.fn(async () => new Response("# README"));
    expect(await fetchExtensionContent("https://open-vsx.org/readme.md", {
      signal: controller.signal, fetchImpl,
    })).toBe("# README");
    expect(fetchImpl).toHaveBeenCalledWith("https://open-vsx.org/readme.md", expect.objectContaining({
      credentials: "omit", signal: controller.signal,
    }));
  });

  it("rejects unsafe URLs before making a request", async () => {
    const fetchImpl = vi.fn();
    await expect(fetchExtensionContent("file:///secret", { fetchImpl })).rejects.toMatchObject({ code: "malformed-response" });
    expect(fetchImpl).not.toHaveBeenCalled();
  });

  it("bounds the actual body even without a content-length header", async () => {
    await expect(fetchExtensionContent("https://open-vsx.org/readme", {
      maxBytes: 4, fetchImpl: async () => new Response("12345"),
    })).rejects.toMatchObject({ code: "malformed-response" });
  });

  it("rejects oversized declared bodies without reading them", async () => {
    const response = new Response("small", { headers: { "content-length": "200" } });
    const getReader = vi.spyOn(response.body!, "getReader");
    await expect(fetchExtensionContent("https://open-vsx.org/readme", {
      maxBytes: 10, fetchImpl: async () => response,
    })).rejects.toMatchObject({ code: "malformed-response" });
    expect(getReader).not.toHaveBeenCalled();
  });

  it("returns typed HTTP failures and preserves an abort", async () => {
    await expect(fetchExtensionContent("https://open-vsx.org/readme", {
      fetchImpl: async () => new Response("missing", { status: 404 }),
    })).rejects.toMatchObject({ code: "not-found" });
    const abort = new DOMException("Aborted", "AbortError");
    await expect(fetchExtensionContent("https://open-vsx.org/readme", {
      fetchImpl: async () => { throw abort; },
    })).rejects.toBe(abort);
  });
});
