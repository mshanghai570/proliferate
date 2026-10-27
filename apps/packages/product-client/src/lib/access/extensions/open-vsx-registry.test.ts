import { describe, expect, it, vi } from "vitest";

import { ExtensionRegistryError } from "#product/lib/domain/extensions/extension-registry-error";
import {
  DEFAULT_OPEN_VSX_BASE_URL,
  OpenVsxRegistry,
  parseRetryAfterSeconds,
} from "#product/lib/access/extensions/open-vsx-registry";

const SEARCH_BODY = {
  extensions: [
    {
      description: "Python language support",
      displayName: "Python",
      downloadCount: 100,
      name: "python",
      namespace: "ms-python",
      timestamp: "2026-03-13T04:21:53.228120Z",
      verified: true,
      version: "2026.4.0",
    },
  ],
  offset: 0,
  totalSize: 865,
};

const METADATA_BODY = {
  allVersions: {
    latest: "https://open-vsx.org/api/ms-python/python/latest",
    "2026.4.0": "https://open-vsx.org/api/ms-python/python/2026.4.0",
    "2026.2.0": "https://open-vsx.org/api/ms-python/python/2026.2.0",
  },
  displayName: "Python",
  files: {
    download: "https://open-vsx.org/api/ms-python/python/2026.4.0/file/ms-python.python-2026.4.0.vsix",
    sha256: "https://open-vsx.org/api/ms-python/python/2026.4.0/file/ms-python.python-2026.4.0.sha256",
  },
  name: "python",
  namespace: "ms-python",
  targetPlatform: "universal",
  version: "2026.4.0",
};

function jsonResponse(
  body: unknown,
  init: { status?: number; headers?: Record<string, string> } = {},
): Response {
  return new Response(JSON.stringify(body), {
    status: init.status ?? 200,
    headers: { "content-type": "application/json", ...init.headers },
  });
}

function textResponse(body: string, init: { status?: number; headers?: Record<string, string> } = {}): Response {
  return new Response(body, {
    status: init.status ?? 200,
    headers: { "content-type": "application/json", ...init.headers },
  });
}

interface RecordedRequest {
  url: string;
  headers: Record<string, string>;
}

function createFetchStub(
  handler: (request: RecordedRequest) => Response | Promise<Response>,
): { fetchImpl: (input: string, init?: RequestInit) => Promise<Response>; requests: RecordedRequest[] } {
  const requests: RecordedRequest[] = [];
  return {
    requests,
    fetchImpl: async (input: string, init?: RequestInit) => {
      const request = { url: input, headers: (init?.headers ?? {}) as Record<string, string> };
      requests.push(request);
      return handler(request);
    },
  };
}

describe("OpenVsxRegistry.search", () => {
  it("requests the documented endpoint with only the supplied parameters", async () => {
    const { fetchImpl, requests } = createFetchStub(() => jsonResponse(SEARCH_BODY));
    const registry = new OpenVsxRegistry({ fetchImpl });

    const result = await registry.search({
      query: "python",
      category: "Programming Languages",
      size: 20,
      offset: 40,
      sortBy: "downloadCount",
      sortOrder: "desc",
    });

    const url = new URL(requests[0]!.url);
    expect(`${url.origin}${url.pathname}`).toBe("https://open-vsx.org/api/-/search");
    expect(Object.fromEntries(url.searchParams)).toEqual({
      query: "python",
      category: "Programming Languages",
      size: "20",
      offset: "40",
      sortBy: "downloadCount",
      sortOrder: "desc",
    });
    expect(result.totalSize).toBe(865);
    expect(result.extensions[0]!.id).toBe("ms-python.python");
  });

  it("omits parameters that were not supplied", async () => {
    const { fetchImpl, requests } = createFetchStub(() => jsonResponse(SEARCH_BODY));
    await new OpenVsxRegistry({ fetchImpl }).search({});
    expect(new URL(requests[0]!.url).search).toBe("");
  });

  it("uses only CORS-safelisted request headers", async () => {
    const { fetchImpl, requests } = createFetchStub(() => jsonResponse(SEARCH_BODY));
    await new OpenVsxRegistry({ fetchImpl }).search({});
    expect(requests[0]!.headers).toEqual({ accept: "application/json" });
  });

  it("honors a custom registry origin", async () => {
    const { fetchImpl, requests } = createFetchStub(() => jsonResponse(SEARCH_BODY));
    await new OpenVsxRegistry({ fetchImpl, baseUrl: "https://vsx.internal.example/" }).search({});
    expect(requests[0]!.url.startsWith("https://vsx.internal.example/api/-/search")).toBe(true);
  });
});

describe("OpenVsxRegistry cross-origin requests", () => {
  it("does not send ETag validators that trigger a denied CORS preflight", async () => {
    const { fetchImpl, requests } = createFetchStub(() =>
      jsonResponse(SEARCH_BODY, { headers: { etag: '"abc123"' } }),
    );
    const registry = new OpenVsxRegistry({ fetchImpl });

    const first = await registry.search({ query: "python" });
    const second = await registry.search({ query: "python" });

    expect(requests).toHaveLength(2);
    expect(requests.map((request) => request.headers)).toEqual([
      { accept: "application/json" },
      { accept: "application/json" },
    ]);
    expect(second).toEqual(first);
  });
});

describe("OpenVsxRegistry rate limiting", () => {
  it("waits the registry's Retry-After and then retries", async () => {
    let attempt = 0;
    const { fetchImpl, requests } = createFetchStub(() => {
      attempt += 1;
      if (attempt === 1) {
        return textResponse("slow down", { status: 429, headers: { "retry-after": "2" } });
      }
      return jsonResponse(SEARCH_BODY);
    });
    const sleep = vi.fn(async () => {});
    const registry = new OpenVsxRegistry({ fetchImpl, sleep });

    const result = await registry.search({ query: "python" });

    expect(sleep).toHaveBeenCalledWith(2000);
    expect(requests).toHaveLength(2);
    expect(result.extensions).toHaveLength(1);
  });

  it("raises a typed rate-limited error once the retry budget is spent", async () => {
    const { fetchImpl, requests } = createFetchStub(() =>
      textResponse("slow down", { status: 429, headers: { "retry-after": "7" } }),
    );
    const sleep = vi.fn(async () => {});
    const registry = new OpenVsxRegistry({ fetchImpl, sleep, maxRetryAttempts: 1 });

    const error = await registry.search({ query: "python" }).catch((caught: unknown) => caught);

    expect(error).toBeInstanceOf(ExtensionRegistryError);
    expect((error as ExtensionRegistryError).code).toBe("rate-limited");
    expect((error as ExtensionRegistryError).retryAfterSeconds).toBe(7);
    expect(requests).toHaveLength(2);
  });

  it("falls back to a default delay when Retry-After is unusable", async () => {
    let attempt = 0;
    const { fetchImpl } = createFetchStub(() => {
      attempt += 1;
      return attempt === 1
        ? textResponse("slow down", { status: 429 })
        : jsonResponse(SEARCH_BODY);
    });
    const sleep = vi.fn(async () => {});
    await new OpenVsxRegistry({ fetchImpl, sleep }).search({});
    expect(sleep).toHaveBeenCalledWith(1000);
  });
});

describe("OpenVsxRegistry failures", () => {
  it("maps a 404 to not-found", async () => {
    const { fetchImpl } = createFetchStub(() => textResponse("nope", { status: 404 }));
    const error = await new OpenVsxRegistry({ fetchImpl })
      .getExtension({ namespace: "missing", name: "gone" })
      .catch((caught: unknown) => caught);
    expect((error as ExtensionRegistryError).code).toBe("not-found");
    expect((error as ExtensionRegistryError).status).toBe(404);
  });

  it("maps a server error to http-error", async () => {
    const { fetchImpl } = createFetchStub(() => textResponse("boom", { status: 503 }));
    const error = await new OpenVsxRegistry({ fetchImpl }).search({}).catch((caught: unknown) => caught);
    expect((error as ExtensionRegistryError).code).toBe("http-error");
    expect((error as ExtensionRegistryError).status).toBe(503);
  });

  it("maps invalid JSON to malformed-response", async () => {
    const { fetchImpl } = createFetchStub(() => textResponse("<html>not json</html>"));
    const error = await new OpenVsxRegistry({ fetchImpl }).search({}).catch((caught: unknown) => caught);
    expect((error as ExtensionRegistryError).code).toBe("malformed-response");
  });

  it("maps a transport failure to network-error", async () => {
    const { fetchImpl } = createFetchStub(() => {
      throw new TypeError("fetch failed");
    });
    const error = await new OpenVsxRegistry({ fetchImpl }).search({}).catch((caught: unknown) => caught);
    expect((error as ExtensionRegistryError).code).toBe("network-error");
  });
});

describe("OpenVsxRegistry metadata and packages", () => {
  it("fetches metadata from the namespace/name path", async () => {
    const { fetchImpl, requests } = createFetchStub(() => jsonResponse(METADATA_BODY));
    const metadata = await new OpenVsxRegistry({ fetchImpl }).getExtension({
      namespace: "ms-python",
      name: "python",
    });
    expect(new URL(requests[0]!.url).pathname).toBe("/api/ms-python/python");
    expect(metadata.engines).toEqual({});
    expect(metadata.targetPlatform).toBe("universal");
  });

  it("escapes namespaces and names in the URL", async () => {
    const { fetchImpl, requests } = createFetchStub(() => jsonResponse(METADATA_BODY));
    await new OpenVsxRegistry({ fetchImpl }).getExtension({
      namespace: "weird/space",
      name: "name&more",
    });
    expect(new URL(requests[0]!.url).pathname).toBe("/api/weird%2Fspace/name%26more");
  });

  it("resolves versions from extension metadata", async () => {
    const { fetchImpl, requests } = createFetchStub(() => jsonResponse(METADATA_BODY));
    const versions = await new OpenVsxRegistry({ fetchImpl }).getVersions({
      namespace: "ms-python",
      name: "python",
    });
    expect(requests).toHaveLength(1);
    expect(versions.map((entry) => entry.version)).toEqual(["2026.4.0", "2026.2.0"]);
  });

  it("resolves a package location for an explicit version", async () => {
    const { fetchImpl, requests } = createFetchStub(() => jsonResponse(METADATA_BODY));
    const pkg = await new OpenVsxRegistry({ fetchImpl }).download(
      { namespace: "ms-python", name: "python" },
      "2026.4.0",
    );

    expect(new URL(requests[0]!.url).pathname).toBe("/api/ms-python/python/2026.4.0");
    expect(pkg.version).toBe("2026.4.0");
    expect(pkg.fileName).toBe("ms-python.python-2026.4.0.vsix");
    expect(pkg.downloadUrl).toContain("ms-python.python-2026.4.0.vsix");
    expect(pkg.sha256Url).toContain(".sha256");
    expect(pkg.signatureUrl).toBeNull();
  });
});

describe("parseRetryAfterSeconds", () => {
  it("reads a delay in seconds", () => {
    expect(parseRetryAfterSeconds("12")).toBe(12);
  });

  it("reads an HTTP date relative to now", () => {
    const now = Date.UTC(2026, 0, 1, 0, 0, 0);
    expect(parseRetryAfterSeconds("Fri, 01 Jan 2026 00:00:30 GMT", now)).toBe(30);
  });

  it("returns null for absent or unusable values", () => {
    expect(parseRetryAfterSeconds(null)).toBeNull();
    expect(parseRetryAfterSeconds("   ")).toBeNull();
    expect(parseRetryAfterSeconds("soon")).toBeNull();
  });
});

describe("OpenVsxRegistry defaults", () => {
  it("targets the public registry", () => {
    expect(DEFAULT_OPEN_VSX_BASE_URL).toBe("https://open-vsx.org");
  });
});

describe("marketplace request lifecycle", () => {
  it("passes abort signals and omits application cookies", async () => {
    const controller = new AbortController();
    const fetchImpl = vi.fn(async () => jsonResponse(SEARCH_BODY));
    await new OpenVsxRegistry({ fetchImpl }).search({}, { signal: controller.signal });
    expect(fetchImpl).toHaveBeenCalledWith(expect.any(String), expect.objectContaining({
      signal: controller.signal, credentials: "omit",
    }));
  });

  it("preserves cancellation rather than classifying it as a network failure", async () => {
    const abort = new DOMException("Aborted", "AbortError");
    const registry = new OpenVsxRegistry({ fetchImpl: async () => { throw abort; } });
    await expect(registry.search({})).rejects.toBe(abort);
  });

  it("relies on query caching rather than cross-origin ETag validators", async () => {
    const { fetchImpl, requests } = createFetchStub(() => jsonResponse(SEARCH_BODY, { headers: { etag: '"a"' } }));
    const registry = new OpenVsxRegistry({ fetchImpl });
    await registry.search({ query: "first" });
    await registry.search({ query: "second" });
    await registry.search({ query: "first" });
    expect(requests.every((request) => request.headers["if-none-match"] === undefined)).toBe(true);
  });

  it("surfaces a long Retry-After without leaving a request asleep indefinitely", async () => {
    const sleep = vi.fn(async () => {});
    const registry = new OpenVsxRegistry({
      sleep, fetchImpl: async () => new Response("", { status: 429, headers: { "retry-after": "3600" } }),
    });
    await expect(registry.search({})).rejects.toMatchObject({ code: "rate-limited", retryAfterSeconds: 3600 });
    expect(sleep).not.toHaveBeenCalled();
  });
});

it("rejects non-object registry JSON at the boundary", async () => {
  const registry = new OpenVsxRegistry({ fetchImpl: async () => new Response("null") });
  await expect(registry.search({})).rejects.toMatchObject({ code: "malformed-response" });
});
