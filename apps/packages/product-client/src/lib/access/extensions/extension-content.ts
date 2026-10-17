import { resolveExtensionContentUrl } from "#product/lib/domain/extensions/extension-content";
import { ExtensionRegistryError } from "#product/lib/domain/extensions/extension-registry-error";

export interface ExtensionContentOptions {
  signal?: AbortSignal;
  maxBytes?: number;
  fetchImpl?: (input: string, init?: RequestInit) => Promise<Response>;
}

/** Read public text with an actual byte bound, including chunked responses. */
export async function fetchExtensionContent(url: string, options: ExtensionContentOptions = {}): Promise<string> {
  const safeUrl = resolveExtensionContentUrl(url);
  if (!safeUrl) throw new ExtensionRegistryError("malformed-response", "Unsafe extension content URL");
  const maxBytes = options.maxBytes ?? 1024 * 1024;
  const fetchImpl = options.fetchImpl ?? globalThis.fetch;
  let response: Response;
  try {
    response = await fetchImpl(safeUrl, { signal: options.signal, credentials: "omit" });
  } catch (error) {
    if (options.signal?.aborted || (error instanceof Error && error.name === "AbortError")) throw error;
    throw new ExtensionRegistryError("network-error", "Extension content could not be loaded");
  }
  if (!response.ok) {
    await response.body?.cancel();
    throw new ExtensionRegistryError(response.status === 404 ? "not-found" : "http-error", "Extension content is unavailable", { status: response.status });
  }
  return readBoundedExtensionText(response, maxBytes, options.signal);
}

export async function readBoundedExtensionText(response: Response, maxBytes: number, signal?: AbortSignal): Promise<string> {
  if (Number(response.headers.get("content-length")) > maxBytes) {
    await response.body?.cancel();
    throw new ExtensionRegistryError("malformed-response", "Extension content exceeds the size limit");
  }
  const reader = response.body?.getReader();
  if (!reader) return "";
  const decoder = new TextDecoder();
  const chunks: string[] = [];
  let total = 0;
  try {
    while (true) {
      signal?.throwIfAborted();
      const { value, done } = await reader.read();
      if (done) break;
      total += value.byteLength;
      if (total > maxBytes) {
        await reader.cancel();
        throw new ExtensionRegistryError("malformed-response", "Extension content exceeds the size limit");
      }
      chunks.push(decoder.decode(value, { stream: true }));
    }
    chunks.push(decoder.decode());
    return chunks.join("");
  } finally {
    reader.releaseLock();
  }
}
