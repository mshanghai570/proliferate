/**
 * The registry-agnostic extension vocabulary and the `ExtensionRegistry` port.
 *
 * Pure declarations and plain data only: no fetch, no React, no stores, no SDK
 * clients, no DOM. A concrete registry (Open VSX today) lives in the access
 * layer under `lib/access/extensions/**` and maps its wire format through the
 * pure mappers in `lib/domain/extensions/**`.
 *
 * Two ideas keep this honest:
 *
 * - An extension is identified by `namespace.name` (the registry's "publisher
 *   id" form). `ExtensionRef` is the pair; `extensionId` renders the string.
 * - `download` resolves *where* a package lives, never its bytes. Fetching and
 *   unpacking a `.vsix` is the runtime's job (it owns the filesystem); the
 *   client only needs a location it can hand over.
 */

/** The registry's supported result orderings (Open VSX `sortBy`). */
export type ExtensionSortBy = "relevance" | "timestamp" | "rating" | "downloadCount";

/** The registry's supported orderings (Open VSX `sortOrder`). */
export type ExtensionSortOrder = "asc" | "desc";

export const EXTENSION_SORT_BY_VALUES: readonly ExtensionSortBy[] = [
  "relevance",
  "timestamp",
  "rating",
  "downloadCount",
];

export const EXTENSION_SORT_ORDER_VALUES: readonly ExtensionSortOrder[] = ["asc", "desc"];

/**
 * Target platforms a registry can expose. Treated as an open set: a registry
 * may return a platform we do not know yet, so models carry `string | null`
 * and these values are only used when *requesting* a platform.
 */
export const EXTENSION_TARGET_PLATFORMS: readonly string[] = [
  "win32-x64",
  "win32-ia32",
  "win32-arm64",
  "linux-x64",
  "linux-arm64",
  "linux-armhf",
  "alpine-x64",
  "alpine-arm64",
  "darwin-x64",
  "darwin-arm64",
  "web",
  "universal",
];

/** Publisher/namespace identity as the registry reports it. */
export interface ExtensionPublisher {
  loginName: string;
  fullName: string | null;
  avatarUrl: string | null;
  homepage: string | null;
  provider: string | null;
}

/** Absolute URLs the registry offers for an extension version's files. */
export interface ExtensionFileUrls {
  download: string | null;
  sha256: string | null;
  signature: string | null;
  publicKey: string | null;
  icon: string | null;
  readme: string | null;
  changelog: string | null;
  license: string | null;
  manifest: string | null;
}

export interface ExtensionRef {
  namespace: string;
  name: string;
}

/** One extension as it appears in a search result. */
export interface ExtensionSummary {
  ref: ExtensionRef;
  /** `namespace.name` — the registry's stable extension id. */
  id: string;
  displayName: string;
  description: string;
  version: string;
  /** ISO-8601 timestamp of the version's publication, or `null`. */
  timestamp: string | null;
  downloadCount: number | null;
  averageRating: number | null;
  reviewCount: number | null;
  verified: boolean;
  deprecated: boolean;
  preview: boolean;
  preRelease: boolean;
  targetPlatform: string | null;
  iconUrl: string | null;
}

export interface ExtensionSearchQuery {
  /** Free-text query. Empty/omitted returns the registry's default ordering. */
  query?: string;
  /** Registry category, e.g. `Programming Languages`. */
  category?: string;
  targetPlatform?: string;
  /** Page size. The registry caps this; do not assume a value is honored. */
  size?: number;
  offset?: number;
  sortBy?: ExtensionSortBy;
  sortOrder?: ExtensionSortOrder;
  /** Include every version of each matching extension in the results. */
  includeAllVersions?: boolean;
}

export interface ExtensionSearchResult {
  extensions: ExtensionSummary[];
  /** Total matches the registry reports, for pagination. */
  totalSize: number;
  offset: number;
}

/** A version of an extension, without its full metadata. */
export interface ExtensionVersion {
  version: string;
  /** Absolute URL of that version's metadata document. */
  url: string;
  /** True for the version the registry reports as current. */
  latest: boolean;
}

/** Full metadata for one extension version. */
export interface ExtensionMetadata extends ExtensionSummary {
  namespaceDisplayName: string | null;
  engines: Record<string, string>;
  categories: string[];
  tags: string[];
  license: string | null;
  repository: string | null;
  homepage: string | null;
  bugs: string | null;
  publisher: ExtensionPublisher | null;
  /** Extension ids this extension depends on, `namespace.name` form. */
  dependencies: string[];
  /** Extension ids bundled inside this package, `namespace.name` form. */
  bundledExtensions: string[];
  allVersions: ExtensionVersion[];
  fileUrls: ExtensionFileUrls;
}

/**
 * Where an extension package lives. This is a *location*, not the package:
 * the runtime performs the download and verification (see the module doc).
 */
export interface ExtensionPackage {
  ref: ExtensionRef;
  id: string;
  version: string;
  /** File name the registry serves, e.g. `ms-python.python-2026.4.0.vsix`. */
  fileName: string;
  downloadUrl: string;
  /** URL of the SHA-256 digest document, when the registry offers one. */
  sha256Url: string | null;
  /** URL of the detached signature archive, when offered. */
  signatureUrl: string | null;
  /** URL of the public key that verifies the signature, when offered. */
  publicKeyUrl: string | null;
  targetPlatform: string | null;
}

/**
 * The port every extension registry implements. Open VSX is the first
 * implementation; private registries and local `.vsix` sources are later
 * adapters behind the same shape.
 */
export interface ExtensionRequestOptions {
  signal?: AbortSignal;
}

export interface ExtensionRegistry {
  search(query: ExtensionSearchQuery, options?: ExtensionRequestOptions): Promise<ExtensionSearchResult>;

  getExtension(ref: ExtensionRef, options?: ExtensionRequestOptions): Promise<ExtensionMetadata>;

  getVersions(ref: ExtensionRef, options?: ExtensionRequestOptions): Promise<ExtensionVersion[]>;

  download(ref: ExtensionRef, version: string, options?: ExtensionRequestOptions): Promise<ExtensionPackage>;
}

/** `namespace.name`, the registry's stable extension id. */
export function extensionId(ref: ExtensionRef): string {
  return `${ref.namespace}.${ref.name}`;
}

/** The offset of the next page, or `null` when the result set is complete. */
export function nextPageOffset(result: ExtensionSearchResult, pageSize: number): number | null {
  const next = result.offset + pageSize;
  return next < result.totalSize ? next : null;
}
