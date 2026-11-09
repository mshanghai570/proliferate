/**
 * Open VSX registry adapter.
 *
 * Implements the `ExtensionRegistry` port against the registry's documented
 * REST API (<https://open-vsx.org/v3/api-docs>). It is the raw transport half of
 * the extension surface: no React, no query client, no product state — fetch in,
 * domain models out, failures as `ExtensionRegistryError`.
 *
 * Two behaviors here are deliberate, not incidental:
 *
 * - **Browser-compatible requests.** Requests use only the safelisted Accept
 *   header. Open VSX does not allow `If-None-Match` in its CORS preflight, so
 *   conditional requests would fail in a browser/WebView even when reachable.
 *   TanStack Query owns response caching above this transport adapter.
 * - **Rate-limit backoff.** Responses carry `X-RateLimit-*`, and a 429 carries
 *   `Retry-After`. A 429 is retried a bounded number of times after the
 *   registry's own delay; exhausting the budget raises `rate-limited` with the
 *   delay attached, so the caller can surface a real state instead of hanging.
 * - **Locations, not bytes.** `download` resolves the package URL the registry
 *   advertises. The runtime fetches and unpacks it — the client never holds a
 *   `.vsix`.
 */

import { readBoundedExtensionText } from "#product/lib/access/extensions/extension-content";
import { ExtensionRegistryError } from "#product/lib/domain/extensions/extension-registry-error";
import type {
  ExtensionMetadata,
  ExtensionPackage,
  ExtensionRef,
  ExtensionRequestOptions,
  ExtensionRegistry,
  ExtensionSearchQuery,
  ExtensionSearchResult,
  ExtensionVersion,
} from "#product/lib/domain/extensions/extension-models";
import {
  mapMetadata,
  mapPackage,
  mapSearchResponse,
} from "#product/lib/domain/extensions/open-vsx-mapping";
import type {
  OpenVsxExtensionMetadata,
  OpenVsxSearchResponse,
} from "#product/lib/domain/extensions/open-vsx-wire";

export const DEFAULT_OPEN_VSX_BASE_URL = "https://open-vsx.org";

/** Delay used when a 429 arrives without a usable `Retry-After` header. */
export const DEFAULT_RETRY_AFTER_SECONDS = 1;

type FetchLike = (input: string, init?: RequestInit) => Promise<Response>;

interface TextResponse {
  status: number;
  text: string;
}

export interface OpenVsxRegistryOptions {
  /** Registry origin. Defaults to `https://open-vsx.org`. */
  baseUrl?: string;
  /** Inject for tests; defaults to the global `fetch`. */
  fetchImpl?: FetchLike;
  /** Extra attempts after a 429. Defaults to 2 (three tries total). */
  maxRetryAttempts?: number;
  /** Inject for tests; defaults to a `setTimeout`-backed sleep. */
  sleep?: (milliseconds: number, signal?: AbortSignal) => Promise<void>;
}

/**
 * Parse a `Retry-After` header, which may be a delay in seconds or an HTTP
 * date. Returns `null` when the header is absent or unusable.
 */
export function parseRetryAfterSeconds(value: string | null, now: number = Date.now()): number | null {
  if (value === null) return null;
  const trimmed = value.trim();
  if (trimmed.length === 0) return null;
  const seconds = Number(trimmed);
  if (Number.isFinite(seconds) && seconds >= 0) return seconds;
  const timestamp = Date.parse(trimmed);
  if (!Number.isNaN(timestamp)) return Math.max(0, Math.ceil((timestamp - now) / 1000));
  return null;
}

function defaultSleep(milliseconds: number, signal?: AbortSignal): Promise<void> {
  return new Promise((resolve, reject) => {
    signal?.throwIfAborted();
    const onAbort = () => {
      clearTimeout(timer);
      signal?.removeEventListener("abort", onAbort);
      reject(signal?.reason);
    };
    const timer = setTimeout(() => {
      signal?.removeEventListener("abort", onAbort);
      resolve();
    }, milliseconds);
    signal?.addEventListener("abort", onAbort, { once: true });
  });
}

export class OpenVsxRegistry implements ExtensionRegistry {
  private readonly baseUrl: string;
  private readonly fetchImpl: FetchLike;
  private readonly maxRetryAttempts: number;
  private readonly sleep: (milliseconds: number, signal?: AbortSignal) => Promise<void>;

  constructor(options: OpenVsxRegistryOptions = {}) {
    this.baseUrl = (options.baseUrl ?? DEFAULT_OPEN_VSX_BASE_URL).replace(/\/+$/, "");
    this.fetchImpl = options.fetchImpl ?? ((input, init) => globalThis.fetch(input, init));
    this.maxRetryAttempts = options.maxRetryAttempts ?? 2;
    this.sleep = options.sleep ?? defaultSleep;
  }

  async search(query: ExtensionSearchQuery, options: ExtensionRequestOptions = {}): Promise<ExtensionSearchResult> {
    const url = this.searchUrl(query);
    const raw = await this.requestJson<OpenVsxSearchResponse>(url, "Extension search", options.signal);
    return mapSearchResponse(raw, this.baseUrl);
  }

  async getExtension(ref: ExtensionRef, options: ExtensionRequestOptions = {}): Promise<ExtensionMetadata> {
    const url = this.apiUrl(ref.namespace, ref.name);
    const raw = await this.requestJson<OpenVsxExtensionMetadata>(url, `Extension ${ref.namespace}.${ref.name}`, options.signal);
    return mapMetadata(raw, this.baseUrl);
  }

  async getVersions(ref: ExtensionRef, options: ExtensionRequestOptions = {}): Promise<ExtensionVersion[]> {
    const metadata = await this.getExtension(ref, options);
    return metadata.allVersions;
  }

  async download(ref: ExtensionRef, version: string, options: ExtensionRequestOptions = {}): Promise<ExtensionPackage> {
    const url = this.apiUrl(ref.namespace, ref.name, version);
    const raw = await this.requestJson<OpenVsxExtensionMetadata>(
      url,
      `Extension ${ref.namespace}.${ref.name}@${version}`,
      options.signal,
    );
    const metadata = mapMetadata(raw, this.baseUrl);
    return mapPackage(metadata, version, ref);
  }

  private apiUrl(...segments: string[]): string {
    return `${this.baseUrl}/api/${segments.map((segment) => encodeURIComponent(segment)).join("/")}`;
  }

  private searchUrl(query: ExtensionSearchQuery): string {
    const url = new URL(this.apiUrl("-", "search"));
    const params = url.searchParams;
    if (query.query) params.set("query", query.query);
    if (query.category) params.set("category", query.category);
    if (query.targetPlatform) params.set("targetPlatform", query.targetPlatform);
    if (query.size !== undefined) params.set("size", String(query.size));
    if (query.offset !== undefined) params.set("offset", String(query.offset));
    if (query.sortBy) params.set("sortBy", query.sortBy);
    if (query.sortOrder) params.set("sortOrder", query.sortOrder);
    if (query.includeAllVersions !== undefined) {
      params.set("includeAllVersions", String(query.includeAllVersions));
    }
    return url.toString();
  }

  private async requestJson<T>(url: string, what: string, signal?: AbortSignal): Promise<T> {
    const { status, text } = await this.requestText(url, signal);
    if (status === 404) {
      throw new ExtensionRegistryError("not-found", `${what} was not found`, { status });
    }
    if (status < 200 || status >= 300) {
      throw new ExtensionRegistryError("http-error", `${what} failed with status ${status}`, { status });
    }
    try {
      const raw: unknown = JSON.parse(text);
      if (raw === null || typeof raw !== "object" || Array.isArray(raw)) {
        throw new Error("Registry response must be an object");
      }
      return raw as T;
    } catch {
      throw new ExtensionRegistryError("malformed-response", `${what} returned invalid JSON`, { status });
    }
  }

  private async requestText(url: string, signal?: AbortSignal): Promise<TextResponse> {
    for (let attempt = 0; ; attempt += 1) {
      signal?.throwIfAborted();
      const headers: Record<string, string> = { accept: "application/json" };

      let response: Response;
      try {
        response = await this.fetchImpl(url, { headers, signal, credentials: "omit" });
      } catch (error) {
        if (signal?.aborted || (error instanceof Error && error.name === "AbortError")) throw error;
        throw new ExtensionRegistryError("network-error", `Could not reach the registry at ${url}`);
      }

      const text = await readBoundedExtensionText(response, 2 * 1024 * 1024, signal);

      if (response.status === 429) {
        const retryAfterSeconds =
          parseRetryAfterSeconds(response.headers.get("retry-after")) ?? DEFAULT_RETRY_AFTER_SECONDS;
        if (attempt < this.maxRetryAttempts && retryAfterSeconds <= 30) {
          await (signal ? this.sleep(retryAfterSeconds * 1000, signal) : this.sleep(retryAfterSeconds * 1000));
          continue;
        }
        throw new ExtensionRegistryError("rate-limited", `Registry rate limit reached for ${url}`, {
          status: 429,
          retryAfterSeconds,
        });
      }

      return { status: response.status, text };
    }
  }
}

export function createOpenVsxRegistry(options: OpenVsxRegistryOptions = {}): OpenVsxRegistry {
  return new OpenVsxRegistry(options);
}
