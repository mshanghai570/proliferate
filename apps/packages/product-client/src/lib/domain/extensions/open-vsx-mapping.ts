/**
 * Pure Open VSX wire → domain mapping.
 *
 * The single place that decides what the registry's JSON means. Mappers are
 * synchronous, dependency-free, and decisionless: they validate, normalize, and
 * resolve relative URLs, and never fetch. Unknown fields are dropped; missing
 * required fields raise `malformed-response` so a changed registry response
 * fails loudly at the boundary instead of producing half-filled cards.
 */

import { ExtensionRegistryError } from "./extension-registry-error";
import type {
  ExtensionFileUrls,
  ExtensionMetadata,
  ExtensionPackage,
  ExtensionPublisher,
  ExtensionRef,
  ExtensionSearchResult,
  ExtensionSummary,
  ExtensionVersion,
} from "./extension-models";
import { extensionId } from "./extension-models";
import type {
  OpenVsxExtensionMetadata,
  OpenVsxFileUrls,
  OpenVsxPublisher,
  OpenVsxSearchEntry,
  OpenVsxSearchResponse,
} from "./open-vsx-wire";

function requireString(value: unknown, field: string): string {
  if (typeof value !== "string" || value.length === 0) {
    throw new ExtensionRegistryError("malformed-response", `Missing required field: ${field}`);
  }
  return value;
}

function optionalString(value: unknown): string | null {
  return typeof value === "string" && value.length > 0 ? value : null;
}

function optionalNumber(value: unknown): number | null {
  return typeof value === "number" && Number.isFinite(value) ? value : null;
}

function optionalBoolean(value: unknown): boolean {
  return value === true;
}

function stringArray(value: unknown): string[] {
  return Array.isArray(value) ? value.filter((entry): entry is string => typeof entry === "string") : [];
}

function stringRecord(value: unknown): Record<string, string> {
  if (!value || typeof value !== "object" || Array.isArray(value)) return {};
  const out: Record<string, string> = {};
  for (const [key, entry] of Object.entries(value as Record<string, unknown>)) {
    if (typeof entry === "string") out[key] = entry;
  }
  return out;
}

/** Resolve a registry URL against the API base; `null` when unusable. */
function resolveUrl(value: unknown, apiBaseUrl: string): string | null {
  const href = optionalString(value);
  if (href === null) return null;
  try {
    return new URL(href, apiBaseUrl).toString();
  } catch {
    return null;
  }
}

export function mapFileUrls(raw: OpenVsxFileUrls | null | undefined, apiBaseUrl: string): ExtensionFileUrls {
  return {
    download: resolveUrl(raw?.download, apiBaseUrl),
    sha256: resolveUrl(raw?.sha256, apiBaseUrl),
    signature: resolveUrl(raw?.signature, apiBaseUrl),
    publicKey: resolveUrl(raw?.publicKey, apiBaseUrl),
    icon: resolveUrl(raw?.icon, apiBaseUrl),
    readme: resolveUrl(raw?.readme, apiBaseUrl),
    changelog: resolveUrl(raw?.changelog, apiBaseUrl),
    license: resolveUrl(raw?.license, apiBaseUrl),
    manifest: resolveUrl(raw?.manifest, apiBaseUrl),
  };
}

function mapPublisher(raw: OpenVsxPublisher | null | undefined): ExtensionPublisher | null {
  if (!raw || typeof raw !== "object") return null;
  const loginName = optionalString(raw.loginName);
  if (loginName === null) return null;
  return {
    loginName,
    fullName: optionalString(raw.fullName),
    avatarUrl: optionalString(raw.avatarUrl),
    homepage: optionalString(raw.homepage),
    provider: optionalString(raw.provider),
  };
}

export function mapSummary(raw: OpenVsxSearchEntry, apiBaseUrl: string): ExtensionSummary {
  const namespace = requireString(raw.namespace, "namespace");
  const name = requireString(raw.name, "name");
  const ref: ExtensionRef = { namespace, name };
  return {
    ref,
    id: extensionId(ref),
    displayName: optionalString(raw.displayName) ?? name,
    description: optionalString(raw.description) ?? "",
    version: requireString(raw.version, "version"),
    timestamp: optionalString(raw.timestamp),
    downloadCount: optionalNumber(raw.downloadCount),
    averageRating: optionalNumber(raw.averageRating),
    reviewCount: optionalNumber(raw.reviewCount),
    verified: optionalBoolean(raw.verified),
    deprecated: optionalBoolean(raw.deprecated),
    preview: optionalBoolean(raw.preview),
    // `preRelease` is absent from search entries; only metadata carries it.
    preRelease: false,
    targetPlatform: optionalString(raw.targetPlatform),
    iconUrl: resolveUrl(raw.files?.icon, apiBaseUrl),
  };
}

export function mapSearchResponse(raw: OpenVsxSearchResponse, apiBaseUrl: string): ExtensionSearchResult {
  if (!Array.isArray(raw.extensions)) {
    throw new ExtensionRegistryError(
      "malformed-response",
      "Search response is missing its extensions array",
    );
  }
  return {
    extensions: (raw.extensions as OpenVsxSearchEntry[]).map((entry) => mapSummary(entry, apiBaseUrl)),
    totalSize: optionalNumber(raw.totalSize) ?? 0,
    offset: optionalNumber(raw.offset) ?? 0,
  };
}

export function mapVersions(raw: OpenVsxExtensionMetadata): ExtensionVersion[] {
  const allVersions = raw.allVersions;
  if (!allVersions || typeof allVersions !== "object" || Array.isArray(allVersions)) return [];
  const latest = typeof raw.version === "string" ? raw.version : null;
  const versions: ExtensionVersion[] = [];
  for (const [version, url] of Object.entries(allVersions as Record<string, unknown>)) {
    // The registry includes a `latest` alias that is not a real version.
    if (version === "latest" || typeof url !== "string") continue;
    versions.push({ version, url, latest: version === latest });
  }
  return versions;
}

export function mapMetadata(raw: OpenVsxExtensionMetadata, apiBaseUrl: string): ExtensionMetadata {
  const summary = mapSummary(raw, apiBaseUrl);
  return {
    ...summary,
    preRelease: optionalBoolean(raw.preRelease),
    namespaceDisplayName: optionalString(raw.namespaceDisplayName),
    engines: stringRecord(raw.engines),
    categories: stringArray(raw.categories),
    tags: stringArray(raw.tags),
    license: optionalString(raw.license),
    repository: optionalString(raw.repository),
    homepage: optionalString(raw.homepage),
    bugs: optionalString(raw.bugs),
    publisher: mapPublisher(raw.publishedBy),
    dependencies: stringArray(raw.dependencies),
    bundledExtensions: stringArray(raw.bundledExtensions),
    allVersions: mapVersions(raw),
    fileUrls: mapFileUrls(raw.files, apiBaseUrl),
  };
}

function fileNameFromUrl(url: string): string {
  const withoutQuery = url.split("?")[0] ?? url;
  const segments = withoutQuery.split("/");
  return segments[segments.length - 1] ?? "";
}

export function mapPackage(
  metadata: ExtensionMetadata,
  version: string,
  ref: ExtensionRef,
): ExtensionPackage {
  const downloadUrl = metadata.fileUrls.download;
  if (downloadUrl === null) {
    throw new ExtensionRegistryError(
      "malformed-response",
      `Extension ${extensionId(ref)} ${version} has no downloadable package`,
    );
  }
  return {
    ref,
    id: extensionId(ref),
    version,
    fileName: fileNameFromUrl(downloadUrl),
    downloadUrl,
    sha256Url: metadata.fileUrls.sha256,
    signatureUrl: metadata.fileUrls.signature,
    publicKeyUrl: metadata.fileUrls.publicKey,
    targetPlatform: metadata.targetPlatform,
  };
}
