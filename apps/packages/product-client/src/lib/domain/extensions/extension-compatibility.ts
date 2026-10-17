/**
 * Can this extension run here? Pure rules, no I/O.
 *
 * The governing constraint is that we must never claim compatibility we have
 * not demonstrated, so every rule is three-valued where it cannot be sure:
 * `satisfied`, `unsatisfied`, or `indeterminate`. Only the first clears a
 * blocker; "I could not tell" blocks just like "no", and is reported
 * separately so the UI can say which one it is.
 *
 * Version comparison is `compareSemver` from `lib/domain/capabilities`, the
 * module that already owns desktop/server version comparison. This file parses
 * *ranges* on top of it and does not re-implement ordering.
 */

import { compareSemver } from "#product/lib/domain/capabilities/version-compat";
import type { ExtensionCapability, ExtensionManifest } from "./extension-manifest";

// ---------------------------------------------------------------------------
// Host platform targeting
// ---------------------------------------------------------------------------

/** The machine the IDE is running on, in registry target-platform terms. */
export interface ExtensionHostOs {
  os: "darwin" | "win32" | "linux" | "web";
  arch: "x64" | "arm64" | "armhf" | "ia32";
}

/**
 * Map a host to the registry's target-platform string (`darwin-arm64`,
 * `win32-x64`, `web`, …). Linux distribution flavour (glibc vs musl) is not
 * detected, so a musl host is reported as `linux-*`; that can over-report
 * compatibility for platform-specific packages and is tracked as a known gap.
 */
export function resolveHostTargetPlatform(host: ExtensionHostOs): string {
  if (host.os === "web") return "web";
  return `${host.os}-${host.arch}`;
}

/**
 * Pick the best platform variant from the ones an extension publishes.
 * Prefers an exact host match, then `universal`; returns `null` when the
 * extension offers nothing that can run here.
 */
export function selectCompatibleTargetPlatform(
  availablePlatforms: readonly string[],
  hostPlatform: string,
): string | null {
  if (availablePlatforms.includes(hostPlatform)) return hostPlatform;
  if (availablePlatforms.includes("universal")) return "universal";
  return null;
}

// ---------------------------------------------------------------------------
// Engine requirements (`engines.vscode`)
// ---------------------------------------------------------------------------

export type EngineRequirementInterpretation = "satisfied" | "unsatisfied" | "indeterminate";

/**
 * A single range token: optional operator, then a `major.minor.patch` core.
 * Hyphen ranges (`1.2.3 - 2.0.0`), `||` unions, `x`-wildcards mid-version, and
 * npm aliases are intentionally unmodelled and yield `indeterminate` rather
 * than a guessed answer.
 */
const RANGE_TOKEN = /^(>=|<=|>|<|=|\^|~)?v?(\d+)\.(\d+)\.(\d+)$/;

/**
 * Split a range into evaluable units, keeping hyphen ranges (`1.2.3 - 2.0.0`)
 * together as one unit. Splitting them apart would read the lower bound as an
 * exact-match token and report a confident, wrong `unsatisfied`.
 */
function tokenizeRange(range: string): string[] {
  const parts = range.split(/\s+/);
  const units: string[] = [];
  for (let index = 0; index < parts.length; index += 1) {
    const part = parts[index] ?? "";
    const previous = units[units.length - 1] ?? "";
    const next = parts[index + 1];
    if (part === "-" && RANGE_TOKEN.test(previous) && next !== undefined && RANGE_TOKEN.test(next)) {
      units[units.length - 1] = `${previous} - ${next}`;
      index += 1;
      continue;
    }
    units.push(part);
  }
  return units;
}

function interpretRangeToken(token: string, engineVersion: string): boolean | null {
  const match = RANGE_TOKEN.exec(token);
  if (!match) return null;
  const operator = match[1] ?? "=";
  const major = Number(match[2]);
  const minor = Number(match[3]);
  const patch = Number(match[4]);
  const target = `${major}.${minor}.${patch}`;
  const comparison = compareSemver(engineVersion, target);
  if (comparison === null) return null;

  switch (operator) {
    case "=":
      return comparison === 0;
    case ">":
      return comparison > 0;
    case ">=":
      return comparison >= 0;
    case "<":
      return comparison < 0;
    case "<=":
      return comparison <= 0;
    case "^": {
      if (comparison < 0) return false;
      // npm caret semantics: `^0.5.0` stays inside the minor line.
      const upper = major === 0 ? `0.${minor + 1}.0` : `${major + 1}.0.0`;
      const upperComparison = compareSemver(engineVersion, upper);
      return upperComparison === null ? null : upperComparison < 0;
    }
    case "~": {
      if (comparison < 0) return false;
      const upperComparison = compareSemver(engineVersion, `${major}.${minor + 1}.0`);
      return upperComparison === null ? null : upperComparison < 0;
    }
    default:
      return null;
  }
}

/**
 * Does `engineVersion` satisfy a declared `engines.vscode` range?
 *
 * A `null` requirement (the manifest declares none) is satisfied. A declared
 * range we cannot fully model is `indeterminate`, never assumed true.
 */
export function interpretEngineRequirement(
  requirement: string | null,
  engineVersion: string,
): EngineRequirementInterpretation {
  if (requirement === null) return "satisfied";
  const trimmed = requirement.trim();
  if (trimmed === "" || trimmed === "*" || trimmed === "x" || trimmed === "X") return "satisfied";
  if (trimmed.includes("||")) return "indeterminate";

  let anyUnsatisfied = false;
  let anyIndeterminate = false;
  for (const token of tokenizeRange(trimmed)) {
    const result = interpretRangeToken(token, engineVersion);
    if (result === null) anyIndeterminate = true;
    else if (!result) anyUnsatisfied = true;
  }
  // "definitely fails" outranks "cannot tell": a reported blocker should be the
  // strongest true statement available.
  if (anyUnsatisfied) return "unsatisfied";
  if (anyIndeterminate) return "indeterminate";
  return "satisfied";
}

// ---------------------------------------------------------------------------
// Host capabilities and the assessment
// ---------------------------------------------------------------------------

/** Which extension capabilities this IDE's host actually implements. */
export interface ExtensionHostCapabilities {
  /**
   * The VS Code API version the host implements. `"0.0.0"` means none — the
   * honest baseline while no extension host exists.
   *
   * Raise this only when the corresponding surface demonstrably works; it is
   * the number every `engines.vscode` requirement is judged against.
   */
  engineVersion: string;
  supported: readonly ExtensionCapability[];
}

/**
 * Today's reality: no extension host, so no capability is implemented. Every
 * code-bearing extension is reported incompatible rather than silently
 * installed and left broken. Update this constant only alongside the host work
 * that makes a capability real.
 */
export const EMPTY_EXTENSION_HOST_CAPABILITIES: ExtensionHostCapabilities = {
  engineVersion: "0.0.0",
  supported: [],
};

export function isCapabilitySupported(
  host: ExtensionHostCapabilities,
  capability: ExtensionCapability,
): boolean {
  return host.supported.includes(capability);
}

export type ExtensionCompatibilityBlocker =
  | { kind: "engine-version"; requirement: string; engineVersion: string }
  | { kind: "engine-unknown"; requirement: string; engineVersion: string }
  | { kind: "platform"; requiredPlatform: string; hostPlatform: string }
  | { kind: "unsupported-api"; capability: ExtensionCapability };

export interface ExtensionCompatibilityAssessment {
  verdict: "compatible" | "incompatible";
  engineInterpretation: EngineRequirementInterpretation;
  supportedCapabilities: ExtensionCapability[];
  unsupportedCapabilities: ExtensionCapability[];
  /** Deterministic order: engine, platform, then capabilities in vocabulary order. */
  blockers: ExtensionCompatibilityBlocker[];
}

export interface ExtensionCompatibilityInput {
  /** The platform variant we would install, when the registry reports one. */
  requiredPlatform?: string | null;
  hostPlatform?: string | null;
}

/**
 * Decide whether an extension can run given the host's real capabilities.
 *
 * Capabilities are reported for both outcomes so the UI can show what an
 * extension *would* contribute once its unsupported APIs exist, instead of a
 * bare refusal.
 */
export function assessExtensionCompatibility(
  manifest: ExtensionManifest,
  host: ExtensionHostCapabilities,
  input: ExtensionCompatibilityInput = {},
): ExtensionCompatibilityAssessment {
  const supportedCapabilities = manifest.contributes.filter((capability) =>
    isCapabilitySupported(host, capability),
  );
  const unsupportedCapabilities = manifest.contributes.filter(
    (capability) => !isCapabilitySupported(host, capability),
  );

  const engineInterpretation = interpretEngineRequirement(manifest.engineRequirement, host.engineVersion);
  const blockers: ExtensionCompatibilityBlocker[] = [];

  if (engineInterpretation === "unsatisfied" && manifest.engineRequirement !== null) {
    blockers.push({
      kind: "engine-version",
      requirement: manifest.engineRequirement,
      engineVersion: host.engineVersion,
    });
  } else if (engineInterpretation === "indeterminate" && manifest.engineRequirement !== null) {
    blockers.push({
      kind: "engine-unknown",
      requirement: manifest.engineRequirement,
      engineVersion: host.engineVersion,
    });
  }

  const requiredPlatform = input.requiredPlatform ?? null;
  const hostPlatform = input.hostPlatform ?? null;
  const declaresPortablePlatform = requiredPlatform === null || requiredPlatform === "universal";
  if (!declaresPortablePlatform && hostPlatform !== null && requiredPlatform !== hostPlatform) {
    blockers.push({ kind: "platform", requiredPlatform, hostPlatform });
  }

  for (const capability of unsupportedCapabilities) {
    blockers.push({ kind: "unsupported-api", capability });
  }

  return {
    verdict: blockers.length === 0 ? "compatible" : "incompatible",
    engineInterpretation,
    supportedCapabilities,
    unsupportedCapabilities,
    blockers,
  };
}
