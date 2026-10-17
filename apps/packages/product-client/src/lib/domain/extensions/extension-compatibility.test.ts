import { describe, expect, it } from "vitest";

import {
  EMPTY_EXTENSION_HOST_CAPABILITIES,
  assessExtensionCompatibility,
  interpretEngineRequirement,
  resolveHostTargetPlatform,
  selectCompatibleTargetPlatform,
  type ExtensionHostCapabilities,
} from "#product/lib/domain/extensions/extension-compatibility";
import { parseExtensionManifest } from "#product/lib/domain/extensions/extension-manifest";

function host(supported: ExtensionHostCapabilities["supported"], engineVersion = "1.99.0"): ExtensionHostCapabilities {
  return { engineVersion, supported };
}

const THEME_EXTENSION = parseExtensionManifest(
  { contributes: { themes: [{ id: "dark", path: "./d.json" }] }, name: "theme", version: "1.0.0" },
  "a.theme@1.0.0",
);

const CODE_EXTENSION = parseExtensionManifest(
  {
    engines: { vscode: "^1.95.0" },
    main: "./out/extension.js",
    name: "code",
    version: "1.0.0",
  },
  "a.code@1.0.0",
);

describe("resolveHostTargetPlatform", () => {
  it("maps host os and arch to registry target strings", () => {
    expect(resolveHostTargetPlatform({ os: "darwin", arch: "arm64" })).toBe("darwin-arm64");
    expect(resolveHostTargetPlatform({ os: "darwin", arch: "x64" })).toBe("darwin-x64");
    expect(resolveHostTargetPlatform({ os: "win32", arch: "x64" })).toBe("win32-x64");
    expect(resolveHostTargetPlatform({ os: "linux", arch: "armhf" })).toBe("linux-armhf");
  });

  it("reports the web host as the web platform", () => {
    expect(resolveHostTargetPlatform({ os: "web", arch: "x64" })).toBe("web");
  });
});

describe("selectCompatibleTargetPlatform", () => {
  it("prefers an exact host match", () => {
    expect(selectCompatibleTargetPlatform(["linux-x64", "darwin-arm64"], "darwin-arm64")).toBe(
      "darwin-arm64",
    );
  });

  it("falls back to universal", () => {
    expect(selectCompatibleTargetPlatform(["universal"], "darwin-arm64")).toBe("universal");
  });

  it("returns null when nothing can run here", () => {
    expect(selectCompatibleTargetPlatform(["win32-x64"], "darwin-arm64")).toBeNull();
    expect(selectCompatibleTargetPlatform([], "darwin-arm64")).toBeNull();
  });
});

describe("interpretEngineRequirement", () => {
  it("treats an absent or open requirement as satisfied", () => {
    expect(interpretEngineRequirement(null, "0.0.0")).toBe("satisfied");
    expect(interpretEngineRequirement("*", "0.0.0")).toBe("satisfied");
    expect(interpretEngineRequirement("x", "0.0.0")).toBe("satisfied");
  });

  it("resolves caret ranges", () => {
    expect(interpretEngineRequirement("^1.95.0", "1.99.0")).toBe("satisfied");
    expect(interpretEngineRequirement("^1.95.0", "1.95.0")).toBe("satisfied");
    expect(interpretEngineRequirement("^1.95.0", "1.90.0")).toBe("unsatisfied");
    expect(interpretEngineRequirement("^1.95.0", "2.0.0")).toBe("unsatisfied");
  });

  it("applies npm caret semantics inside the zero major line", () => {
    expect(interpretEngineRequirement("^0.5.0", "0.5.9")).toBe("satisfied");
    expect(interpretEngineRequirement("^0.5.0", "0.6.0")).toBe("unsatisfied");
  });

  it("resolves tilde ranges", () => {
    expect(interpretEngineRequirement("~1.95.0", "1.95.7")).toBe("satisfied");
    expect(interpretEngineRequirement("~1.95.0", "1.96.0")).toBe("unsatisfied");
  });

  it("resolves comparison operators", () => {
    expect(interpretEngineRequirement(">=1.85.0", "1.95.0")).toBe("satisfied");
    expect(interpretEngineRequirement(">1.95.0", "1.95.0")).toBe("unsatisfied");
    expect(interpretEngineRequirement("<2.0.0", "1.95.0")).toBe("satisfied");
    expect(interpretEngineRequirement("<=1.95.0", "1.96.0")).toBe("unsatisfied");
    expect(interpretEngineRequirement("=1.95.0", "1.95.0")).toBe("satisfied");
  });

  it("resolves compound ranges", () => {
    expect(interpretEngineRequirement(">=1.85.0 <2.0.0", "1.95.0")).toBe("satisfied");
    expect(interpretEngineRequirement(">=1.85.0 <2.0.0", "2.1.0")).toBe("unsatisfied");
  });

  it("reports ranges it cannot model as indeterminate rather than guessing", () => {
    expect(interpretEngineRequirement("1.85.0 - 1.99.0", "1.95.0")).toBe("indeterminate");
    expect(interpretEngineRequirement("^1.85.0 || ^2.0.0", "1.95.0")).toBe("indeterminate");
    expect(interpretEngineRequirement("latest", "1.95.0")).toBe("indeterminate");
  });

  it("prefers a definite failure over an inconclusive token in a compound range", () => {
    expect(interpretEngineRequirement(">=3.0.0 1.2.3 - 2.0.0", "1.95.0")).toBe("unsatisfied");
  });

  it("reports indeterminate against an unparseable engine version", () => {
    expect(interpretEngineRequirement("^1.95.0", "dev")).toBe("indeterminate");
  });
});

describe("assessExtensionCompatibility", () => {
  it("reports every capability unsupported against a host with no extension host", () => {
    const assessment = assessExtensionCompatibility(CODE_EXTENSION, EMPTY_EXTENSION_HOST_CAPABILITIES);

    expect(assessment.verdict).toBe("incompatible");
    expect(assessment.engineInterpretation).toBe("unsatisfied");
    expect(assessment.supportedCapabilities).toEqual([]);
    expect(assessment.unsupportedCapabilities).toEqual(["extension-code"]);
    expect(assessment.blockers).toEqual([
      { kind: "engine-version", requirement: "^1.95.0", engineVersion: "0.0.0" },
      { kind: "unsupported-api", capability: "extension-code" },
    ]);
  });

  it("is compatible when the engine is satisfied and every capability is supported", () => {
    const assessment = assessExtensionCompatibility(CODE_EXTENSION, host(["extension-code"]));

    expect(assessment.verdict).toBe("compatible");
    expect(assessment.blockers).toEqual([]);
    expect(assessment.supportedCapabilities).toEqual(["extension-code"]);
    expect(assessment.unsupportedCapabilities).toEqual([]);
  });

  it("blocks on an inconclusive engine requirement instead of assuming support", () => {
    const extension = parseExtensionManifest(
      { engines: { vscode: "1.85.0 - 1.99.0" }, main: "./m.js", name: "x", version: "1" },
      "a.x@1",
    );
    const assessment = assessExtensionCompatibility(extension, host(["extension-code"]));

    expect(assessment.engineInterpretation).toBe("indeterminate");
    expect(assessment.verdict).toBe("incompatible");
    expect(assessment.blockers).toEqual([
      { kind: "engine-unknown", requirement: "1.85.0 - 1.99.0", engineVersion: "1.99.0" },
    ]);
  });

  it("splits supported from unsupported capabilities without hiding either", () => {
    const extension = parseExtensionManifest(
      {
        contributes: {
          commands: [{ command: "a.b", title: "B" }],
          grammars: [{ language: "python", scopeName: "source.python" }],
          themes: [{ id: "dark", path: "./d.json" }],
        },
        main: "./m.js",
        name: "x",
        version: "1",
      },
      "a.x@1",
    );
    const assessment = assessExtensionCompatibility(extension, host(["themes", "grammars"]));

    expect(assessment.supportedCapabilities).toEqual(["themes", "grammars"]);
    expect(assessment.unsupportedCapabilities).toEqual(["commands", "extension-code"]);
    expect(assessment.blockers).toEqual([
      { kind: "unsupported-api", capability: "commands" },
      { kind: "unsupported-api", capability: "extension-code" },
    ]);
  });

  it("blocks a platform mismatch and accepts universal", () => {
    const mismatch = assessExtensionCompatibility(THEME_EXTENSION, host(["themes"]), {
      hostPlatform: "darwin-arm64",
      requiredPlatform: "win32-x64",
    });
    expect(mismatch.blockers).toEqual([
      { kind: "platform", requiredPlatform: "win32-x64", hostPlatform: "darwin-arm64" },
    ]);

    const universal = assessExtensionCompatibility(THEME_EXTENSION, host(["themes"]), {
      hostPlatform: "darwin-arm64",
      requiredPlatform: "universal",
    });
    expect(universal.verdict).toBe("compatible");
  });

  it("orders blockers deterministically: engine, platform, then capabilities", () => {
    const extension = parseExtensionManifest(
      {
        contributes: { themes: [{ id: "dark", path: "./d.json" }] },
        engines: { vscode: "^99.0.0" },
        name: "x",
        version: "1",
      },
      "a.x@1",
    );
    const assessment = assessExtensionCompatibility(extension, EMPTY_EXTENSION_HOST_CAPABILITIES, {
      hostPlatform: "darwin-arm64",
      requiredPlatform: "win32-x64",
    });

    expect(assessment.blockers.map((blocker) => blocker.kind)).toEqual([
      "engine-version",
      "platform",
      "unsupported-api",
    ]);
  });

  it("does not emit a platform blocker when the host platform is unknown", () => {
    const assessment = assessExtensionCompatibility(THEME_EXTENSION, host(["themes"]), {
      requiredPlatform: "win32-x64",
    });
    expect(assessment.blockers).toEqual([]);
  });
});
