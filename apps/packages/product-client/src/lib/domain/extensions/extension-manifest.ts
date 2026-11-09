/**
 * The parts of a VS Code-format extension manifest this IDE reasons about.
 *
 * A published extension ships a `package.json` manifest inside its `.vsix`
 * (the registry also exposes it at `files.manifest`). This module parses the
 * subset that decides whether the extension can run here and what it
 * contributes — not the whole manifest surface.
 *
 * Pure vocabulary and parsing: no fetch, no React, no stores. Fetching the
 * manifest is the access layer's job; deciding what it means is this module's
 * and `extension-compatibility.ts`'s.
 *
 * Deliberately absent: formatters, linters, and language servers. VS Code does
 * not declare those in `contributes` — an extension implements them through the
 * `languages` API behind a code entrypoint, so they surface here as
 * `extension-code` rather than as a declarable contribution.
 */

import { ExtensionRegistryError } from "./extension-registry-error";

/** Contribution categories, in the order a compatibility report lists them. */
export const EXTENSION_CAPABILITIES = [
  "themes",
  "icon-themes",
  "grammars",
  "languages",
  "snippets",
  "commands",
  "keybindings",
  "configuration",
  "menus",
  "views",
  "tasks",
  "notebooks",
  "debuggers",
  "chat",
  "extension-code",
] as const;

export type ExtensionCapability = (typeof EXTENSION_CAPABILITIES)[number];

export interface ExtensionManifest {
  name: string;
  version: string;
  displayName: string | null;
  /** The `engines.vscode` range, or `null` when the manifest declares none. */
  engineRequirement: string | null;
  /** True when the manifest declares a Node entrypoint (`main`). */
  hasMainEntrypoint: boolean;
  /** True when the manifest declares a web entrypoint (`browser`). */
  hasBrowserEntrypoint: boolean;
  activationEvents: string[];
  /** Extension ids this extension requires, `namespace.name` form. */
  extensionDependencies: string[];
  /** Extension ids this package bundles, `namespace.name` form. */
  extensionPack: string[];
  /** The contribution categories the manifest declares. */
  contributes: ExtensionCapability[];
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function readString(value: unknown): string | null {
  return typeof value === "string" && value.length > 0 ? value : null;
}

function readStringArray(value: unknown): string[] {
  return Array.isArray(value) ? value.filter((entry): entry is string => typeof entry === "string") : [];
}

/** True when the key is present with any non-empty value. */
function declares(contributes: Record<string, unknown>, key: string): boolean {
  const value = contributes[key];
  if (value === undefined || value === null) return false;
  if (Array.isArray(value)) return value.length > 0;
  if (isRecord(value)) return Object.keys(value).length > 0;
  return true;
}

/** Map the declarable `contributes` keys onto this IDE's capability vocabulary. */
const CONTRIBUTION_CAPABILITY_KEYS: ReadonlyArray<readonly [string, ExtensionCapability]> = [
  ["themes", "themes"],
  ["iconThemes", "icon-themes"],
  ["grammars", "grammars"],
  ["languages", "languages"],
  ["snippets", "snippets"],
  ["commands", "commands"],
  ["keybindings", "keybindings"],
  ["configuration", "configuration"],
  ["menus", "menus"],
  ["views", "views"],
  ["viewsContainers", "views"],
  ["taskDefinitions", "tasks"],
  ["problemMatchers", "tasks"],
  ["notebooks", "notebooks"],
  ["debuggers", "debuggers"],
  ["chatParticipants", "chat"],
  ["languageModelTools", "chat"],
  ["languageModels", "chat"],
];

function readContributedCapabilities(contributes: unknown, hasCode: boolean): ExtensionCapability[] {
  const found = new Set<ExtensionCapability>();
  if (isRecord(contributes)) {
    for (const [key, capability] of CONTRIBUTION_CAPABILITY_KEYS) {
      if (declares(contributes, key)) found.add(capability);
    }
  }
  if (hasCode) found.add("extension-code");
  return EXTENSION_CAPABILITIES.filter((capability) => found.has(capability));
}

/**
 * Parse an extension manifest. Throws `malformed-response` when the document is
 * not an object with the identity fields every manifest must carry, so a
 * corrupt package fails at the boundary instead of producing a half-read
 * compatibility verdict.
 */
export function parseExtensionManifest(raw: unknown, source: string): ExtensionManifest {
  if (!isRecord(raw)) {
    throw new ExtensionRegistryError("malformed-response", `Extension manifest is not an object: ${source}`);
  }
  const name = readString(raw.name);
  const version = readString(raw.version);
  if (name === null || version === null) {
    throw new ExtensionRegistryError(
      "malformed-response",
      `Extension manifest is missing name or version: ${source}`,
    );
  }

  const engines = isRecord(raw.engines) ? raw.engines : {};
  const hasMainEntrypoint = readString(raw.main) !== null;
  const hasBrowserEntrypoint = readString(raw.browser) !== null;

  return {
    name,
    version,
    displayName: readString(raw.displayName),
    engineRequirement: readString(engines.vscode),
    hasMainEntrypoint,
    hasBrowserEntrypoint,
    activationEvents: readStringArray(raw.activationEvents),
    extensionDependencies: readStringArray(raw.extensionDependencies),
    extensionPack: readStringArray(raw.extensionPack),
    contributes: readContributedCapabilities(raw.contributes, hasMainEntrypoint || hasBrowserEntrypoint),
  };
}

/**
 * True when the extension asks to be activated unconditionally (`*`). The host
 * must not load such an extension at startup unless the user opted in, so the
 * marketplace surfaces it.
 */
export function requestsStartupActivation(manifest: ExtensionManifest): boolean {
  return manifest.activationEvents.includes("*");
}
