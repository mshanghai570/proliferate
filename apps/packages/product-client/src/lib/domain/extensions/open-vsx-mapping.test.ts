import { describe, expect, it } from "vitest";

import { ExtensionRegistryError } from "#product/lib/domain/extensions/extension-registry-error";
import {
  mapMetadata,
  mapPackage,
  mapSearchResponse,
  mapVersions,
} from "#product/lib/domain/extensions/open-vsx-mapping";
import type { ExtensionMetadata } from "#product/lib/domain/extensions/extension-models";

const BASE = "https://open-vsx.org";

// Shaped from a real `/api/-/search` entry.
const SEARCH_ENTRY = {
  averageRating: 3.875,
  deprecated: false,
  description: "Python language support",
  displayName: "Python",
  downloadCount: 60456807,
  files: {
    download: "https://open-vsx.org/api/ms-python/python/2026.4.0/file/ms-python.python-2026.4.0.vsix",
    icon: "https://open-vsx.org/api/ms-python/python/2026.4.0/file/icon.png",
    sha256: "https://open-vsx.org/api/ms-python/python/2026.4.0/file/ms-python.python-2026.4.0.sha256",
    signature: "https://open-vsx.org/api/ms-python/python/2026.4.0/file/ms-python.python-2026.4.0.sigzip",
  },
  name: "python",
  namespace: "ms-python",
  reviewCount: 16,
  timestamp: "2026-03-13T04:21:53.228120Z",
  url: "https://open-vsx.org/api/ms-python/python",
  verified: true,
  version: "2026.4.0",
};

// Shaped from a real `/api/{namespace}/{extension}` document.
const METADATA = {
  ...SEARCH_ENTRY,
  allVersions: {
    latest: "https://open-vsx.org/api/ms-python/python/latest",
    "2026.4.0": "https://open-vsx.org/api/ms-python/python/2026.4.0",
    "2026.2.0": "https://open-vsx.org/api/ms-python/python/2026.2.0",
  },
  bugs: "https://github.com/microsoft/vscode-python/issues",
  categories: ["Programming Languages", "Debuggers"],
  dependencies: [],
  engines: { vscode: "^1.95.0" },
  homepage: "https://github.com/microsoft/vscode-python",
  license: "MIT",
  namespaceDisplayName: "Microsoft",
  preRelease: false,
  publishedBy: {
    avatarUrl: "https://avatars.githubusercontent.com/u/61870837?v=4",
    fullName: "Open VSX",
    loginName: "open-vsx",
    provider: "github",
  },
  repository: "https://github.com/microsoft/vscode-python.git",
  tags: ["python", "linters"],
  targetPlatform: "universal",
};

describe("mapSearchResponse", () => {
  it("normalizes entries and preserves pagination fields", () => {
    const result = mapSearchResponse({ extensions: [SEARCH_ENTRY], offset: 20, totalSize: 865 }, BASE);

    expect(result.offset).toBe(20);
    expect(result.totalSize).toBe(865);
    expect(result.extensions).toHaveLength(1);
    const [entry] = result.extensions;
    expect(entry).toBeDefined();
    expect(entry!.id).toBe("ms-python.python");
    expect(entry!.ref).toEqual({ namespace: "ms-python", name: "python" });
    expect(entry!.displayName).toBe("Python");
    expect(entry!.downloadCount).toBe(60456807);
    expect(entry!.averageRating).toBe(3.875);
    expect(entry!.verified).toBe(true);
    expect(entry!.deprecated).toBe(false);
    // Search entries do not carry a target platform; only metadata does.
    expect(entry!.targetPlatform).toBeNull();
    expect(entry!.preRelease).toBe(false);
    expect(entry!.iconUrl).toBe(
      "https://open-vsx.org/api/ms-python/python/2026.4.0/file/icon.png",
    );
  });

  it("fails loudly when the extensions array is missing", () => {
    expect(() => mapSearchResponse({ offset: 0, totalSize: 0 }, BASE)).toThrowError(
      ExtensionRegistryError,
    );
  });

  it("fails loudly when an entry is missing its identity", () => {
    expect(() => mapSearchResponse({ extensions: [{ name: "python" }] }, BASE)).toThrowError(
      /Missing required field: namespace/,
    );
  });
});

describe("mapMetadata", () => {
  it("lifts metadata fields, publisher, and resolved file URLs", () => {
    const metadata = mapMetadata(METADATA, BASE);

    expect(metadata.id).toBe("ms-python.python");
    expect(metadata.namespaceDisplayName).toBe("Microsoft");
    expect(metadata.engines).toEqual({ vscode: "^1.95.0" });
    expect(metadata.categories).toEqual(["Programming Languages", "Debuggers"]);
    expect(metadata.tags).toEqual(["python", "linters"]);
    expect(metadata.license).toBe("MIT");
    expect(metadata.publisher?.loginName).toBe("open-vsx");
    expect(metadata.fileUrls.manifest).toBeNull();
    expect(metadata.fileUrls.readme).toBeNull();
    expect(metadata.fileUrls.download).toContain("ms-python.python-2026.4.0.vsix");
    expect(metadata.targetPlatform).toBe("universal");
    expect(metadata.preRelease).toBe(false);
  });

  it("drops unknown fields rather than leaking them into the domain", () => {
    const metadata = mapMetadata({ ...METADATA, someFutureField: "x" } as never, BASE);
    expect(Object.keys(metadata)).not.toContain("someFutureField");
  });

  it("falls back to the package name when no display name is published", () => {
    const metadata = mapMetadata({ ...METADATA, displayName: undefined }, BASE);
    expect(metadata.displayName).toBe("python");
  });
});

describe("mapVersions", () => {
  it("drops the registry's `latest` alias and marks the current version", () => {
    const versions = mapVersions(METADATA);

    expect(versions.map((entry) => entry.version)).toEqual(["2026.4.0", "2026.2.0"]);
    expect(versions.find((entry) => entry.version === "2026.4.0")?.latest).toBe(true);
    expect(versions.find((entry) => entry.version === "2026.2.0")?.latest).toBe(false);
  });

  it("returns an empty list when the registry publishes no version map", () => {
    expect(mapVersions({ ...METADATA, allVersions: undefined })).toEqual([]);
  });
});

describe("mapPackage", () => {
  it("resolves a package location with the file name from the download URL", () => {
    const metadata: ExtensionMetadata = mapMetadata(METADATA, BASE);
    const pkg = mapPackage(metadata, "2026.4.0", { namespace: "ms-python", name: "python" });

    expect(pkg.id).toBe("ms-python.python");
    expect(pkg.version).toBe("2026.4.0");
    expect(pkg.fileName).toBe("ms-python.python-2026.4.0.vsix");
    expect(pkg.sha256Url).toContain(".sha256");
    expect(pkg.signatureUrl).toContain(".sigzip");
    expect(pkg.targetPlatform).toBe("universal");
  });

  it("fails when the registry advertises no downloadable package", () => {
    const metadata: ExtensionMetadata = {
      ...mapMetadata(METADATA, BASE),
      fileUrls: {
        download: null,
        sha256: null,
        signature: null,
        publicKey: null,
        icon: null,
        readme: null,
        changelog: null,
        license: null,
        manifest: null,
      },
    };
    expect(() => mapPackage(metadata, "2026.4.0", metadata.ref)).toThrowError(/no downloadable package/);
  });
});
