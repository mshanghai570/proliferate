/**
 * Raw Open VSX wire shapes.
 *
 * These mirror the registry's JSON exactly and are deliberately permissive:
 * every field is optional and typed `unknown` where the registry's own schema
 * is loose. Nothing outside `open-vsx-mapping.ts` should read these types —
 * domain code consumes the normalized models in `extension-models.ts`, and the
 * mapper is the single place that decides what a missing or surprising field
 * means.
 *
 * Source of truth: <https://open-vsx.org/v3/api-docs> (paths `/api/-/search`,
 * `/api/{namespace}/{extension}`, `/api/{namespace}/{extension}/{version}`).
 */

export interface OpenVsxFileUrls {
  download?: unknown;
  sha256?: unknown;
  signature?: unknown;
  publicKey?: unknown;
  icon?: unknown;
  readme?: unknown;
  changelog?: unknown;
  license?: unknown;
  manifest?: unknown;
  vsixmanifest?: unknown;
}

/** One entry of the `/api/-/search` response. */
export interface OpenVsxSearchEntry {
  namespace?: unknown;
  name?: unknown;
  version?: unknown;
  displayName?: unknown;
  description?: unknown;
  timestamp?: unknown;
  downloadCount?: unknown;
  averageRating?: unknown;
  reviewCount?: unknown;
  verified?: unknown;
  deprecated?: unknown;
  preview?: unknown;
  /** Present on metadata documents; absent from `/api/-/search` entries. */
  targetPlatform?: unknown;
  files?: OpenVsxFileUrls | null;
}

export interface OpenVsxSearchResponse {
  extensions?: unknown;
  offset?: unknown;
  totalSize?: unknown;
}

export interface OpenVsxPublisher {
  loginName?: unknown;
  fullName?: unknown;
  avatarUrl?: unknown;
  homepage?: unknown;
  provider?: unknown;
}

/** `/api/{namespace}/{extension}` and `/api/{namespace}/{extension}/{version}`. */
export interface OpenVsxExtensionMetadata extends OpenVsxSearchEntry {
  /** Present on metadata documents; absent from `/api/-/search` entries. */
  preRelease?: unknown;
  namespaceDisplayName?: unknown;
  engines?: unknown;
  categories?: unknown;
  tags?: unknown;
  license?: unknown;
  repository?: unknown;
  homepage?: unknown;
  bugs?: unknown;
  publishedBy?: OpenVsxPublisher | null;
  /** Extension ids this extension depends on. */
  dependencies?: unknown;
  /** Extension ids bundled in this package. */
  bundledExtensions?: unknown;
  /** Map of version string to that version's metadata URL, plus `latest`. */
  allVersions?: unknown;
}
