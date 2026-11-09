import { describe, expect, it } from "vitest";

import { ExtensionRegistryError } from "#product/lib/domain/extensions/extension-registry-error";
import {
  parseExtensionManifest,
  requestsStartupActivation,
} from "#product/lib/domain/extensions/extension-manifest";

function parse(raw: unknown) {
  return parseExtensionManifest(raw, "publisher.extension@1.0.0");
}

describe("parseExtensionManifest", () => {
  it("reads identity, engine requirement, and entrypoints", () => {
    const manifest = parse({
      browser: "./dist/web.js",
      displayName: "Python",
      engines: { vscode: "^1.95.0" },
      main: "./out/extension.js",
      name: "python",
      version: "2026.4.0",
    });

    expect(manifest.name).toBe("python");
    expect(manifest.version).toBe("2026.4.0");
    expect(manifest.displayName).toBe("Python");
    expect(manifest.engineRequirement).toBe("^1.95.0");
    expect(manifest.hasMainEntrypoint).toBe(true);
    expect(manifest.hasBrowserEntrypoint).toBe(true);
  });

  it("reports no engine requirement when the manifest declares none", () => {
    expect(parse({ name: "x", version: "1.0.0" }).engineRequirement).toBeNull();
  });

  it("maps declarable contributions onto the capability vocabulary", () => {
    const manifest = parse({
      contributes: {
        commands: [{ command: "x.run", title: "Run" }],
        debuggers: [{ type: "node", label: "Node" }],
        grammars: [{ language: "python", scopeName: "source.python" }],
        iconThemes: [{ id: "icons", path: "./icons.json" }],
        snippets: [{ language: "python", path: "./snippets.json" }],
        themes: [{ id: "dark", path: "./dark.json" }],
      },
      name: "x",
      version: "1.0.0",
    });

    // Declarative contributions, in vocabulary order.
    expect(manifest.contributes).toEqual([
      "themes",
      "icon-themes",
      "grammars",
      "snippets",
      "commands",
      "debuggers",
    ]);
    expect(manifest.contributes).not.toContain("extension-code");
  });

  it("treats a code entrypoint as the extension-code capability", () => {
    const node = parse({
      contributes: { themes: [{ id: "dark", path: "./d.json" }] },
      main: "./out/extension.js",
      name: "x",
      version: "1",
    });
    expect(node.contributes).toEqual(["themes", "extension-code"]);

    const web = parse({ browser: "./web.js", name: "x", version: "1" });
    expect(web.contributes).toEqual(["extension-code"]);
  });

  it("dedupes contribution keys that map to the same capability", () => {
    const manifest = parse({
      contributes: {
        views: { explorer: [{ id: "a", name: "A" }] },
        viewsContainers: { activitybar: [{ id: "b" }] },
      },
      name: "x",
      version: "1",
    });
    expect(manifest.contributes).toEqual(["views"]);
  });

  it("does not count empty or unknown contribution keys", () => {
    const manifest = parse({
      contributes: { commands: [], themes: null, someFutureKey: [{ a: 1 }] },
      name: "x",
      version: "1",
    });
    expect(manifest.contributes).toEqual([]);
  });

  it("reads activation events, dependencies, and extension packs", () => {
    const manifest = parse({
      activationEvents: ["*", "onLanguage:python"],
      extensionDependencies: ["ms-python.python"],
      extensionPack: ["a.b", "c.d"],
      name: "x",
      version: "1",
    });

    expect(manifest.activationEvents).toEqual(["*", "onLanguage:python"]);
    expect(manifest.extensionDependencies).toEqual(["ms-python.python"]);
    expect(manifest.extensionPack).toEqual(["a.b", "c.d"]);
    expect(requestsStartupActivation(manifest)).toBe(true);
  });

  it("does not report startup activation without the wildcard event", () => {
    const manifest = parse({ activationEvents: ["onLanguage:python"], name: "x", version: "1" });
    expect(requestsStartupActivation(manifest)).toBe(false);
  });

  it("fails loudly on a manifest that is not an object", () => {
    expect(() => parse("<html>")).toThrowError(ExtensionRegistryError);
  });

  it("fails loudly on a manifest missing name or version", () => {
    expect(() => parse({ version: "1.0.0" })).toThrowError(/missing name or version/);
    expect(() => parse({ name: "x" })).toThrowError(/missing name or version/);
  });
});
