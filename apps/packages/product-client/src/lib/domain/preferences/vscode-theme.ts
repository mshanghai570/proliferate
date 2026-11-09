/**
 * VSCode-compatible theme import. Parses theme JSON (the `colors` object
 * shape) and projects it onto Proliferate's semantic token vocabulary as a
 * runtime override set. The projection is deliberately partial: only the
 * shell colors a theme can carry coherently are mapped, and unset roles keep
 * the mono palette. Syntax token colors (`tokenColors`) are out of scope for
 * the editor palettes.
 */

export interface ImportedThemePreference {
  name: string;
  type: "dark" | "light";
  colors: Record<string, string>;
}

export type ImportedThemeParseResult =
  | { ok: true; theme: ImportedThemePreference }
  | { ok: false; error: string };

const MAX_COLOR_ENTRIES = 256;
const MAX_COLOR_KEY_LENGTH = 96;
const MAX_NAME_LENGTH = 128;

function normalizeHexColor(value: string): string | null {
  const standardHashPattern = /^#[0-9a-f]{8}$|^#[0-9a-f]{6}$/i;
  if (standardHashPattern.test(value)) {
    return value.toLowerCase();
  }
  const threeDigitHashPattern = /^#([0-9a-f])([0-9a-f])([0-9a-f])$/i;
  const short = value.match(threeDigitHashPattern);
  if (short) {
    const [r, g, b] = [short[1]!, short[2]!, short[3]!];
    return `#${r}${r}${g}${g}${b}${b}`.toLowerCase();
  }
  const functionPattern = /^rgba?\(\s*(\d{1,3})\s*,\s*(\d{1,3})\s*,\s*(\d{1,3})\s*(?:,\s*([\d.]+)\s*)?\)$/i;
  const fn = value.match(functionPattern);
  if (fn) {
    const channels = [Number(fn[1]), Number(fn[2]), Number(fn[3])];
    if (channels.some((channel) => channel > 255)) {
      return null;
    }
    const hex = channels.map((channel) => channel.toString(16).padStart(2, "0")).join("");
    if (fn[4] === undefined) {
      return `#${hex}`;
    }
    const alpha = Number.parseFloat(fn[4]);
    if (!Number.isFinite(alpha) || alpha < 0 || alpha > 1) {
      return null;
    }
    const alphaHex = Math.round(alpha * 255)
      .toString(16)
      .padStart(2, "0");
    return `#${hex}${alphaHex}`.toLowerCase();
  }
  return null;
}

function hexLuminance(hex: string): number {
  const body = hex.slice(1).slice(0, 6);
  const channels = [0, 2, 4].map((start) => Number.parseInt(body.slice(start, start + 2), 16) / 255);
  const [r, g, b] = channels.map((channel) => (
    channel <= 0.03928 ? channel / 12.92 : ((channel + 0.055) / 1.055) ** 2.4
  ));
  return 0.2126 * r! + 0.7152 * g! + 0.0722 * b!;
}

function normalizeColorValues(colors: unknown): Record<string, string> | null {
  if (!colors || typeof colors !== "object" || Array.isArray(colors)) {
    return null;
  }
  const entries = Object.entries(colors as Record<string, unknown>);
  if (entries.length === 0 || entries.length > MAX_COLOR_ENTRIES) {
    return null;
  }
  const normalized: Record<string, string> = {};
  for (const [key, raw] of entries) {
    if (
      typeof key !== "string"
      || key.length === 0
      || key.length > MAX_COLOR_KEY_LENGTH
      || !/^[a-zA-Z0-9.]+$/.test(key)
    ) {
      return null;
    }
    if (typeof raw !== "string") {
      continue;
    }
    const value = normalizeHexColor(raw.trim());
    if (value && value !== "#00000000") {
      normalized[key] = value;
    }
  }
  return normalized;
}

function normalizeName(value: unknown): string {
  if (typeof value === "string") {
    const trimmed = value.trim();
    if (trimmed && trimmed.length <= MAX_NAME_LENGTH) {
      return trimmed;
    }
  }
  return "Imported theme";
}

function resolveThemeType(value: unknown, colors: Record<string, string>): "dark" | "light" | null {
  if (value === "dark" || value === "light") {
    return value;
  }
  // Older/wrapper formats may omit `type`; infer the posture from the editor
  // background when it is present and decisive.
  const editorBackground = colors["editor.background"];
  if (editorBackground) {
    return hexLuminance(editorBackground) < 0.35 ? "dark" : "light";
  }
  return null;
}

export function parseVscodeThemeText(text: string): ImportedThemeParseResult {
  let parsed: unknown;
  try {
    parsed = JSON.parse(text);
  } catch {
    return { ok: false, error: "This file is not valid JSON." };
  }
  if (!parsed || typeof parsed !== "object" || Array.isArray(parsed)) {
    return { ok: false, error: "The theme must be a single JSON object." };
  }
  const record = parsed as Record<string, unknown>;
  if (typeof record.colors !== "object" || record.colors === null) {
    return { ok: false, error: "The theme is missing its \"colors\" section." };
  }
  const colors = normalizeColorValues(record.colors);
  if (!colors) {
    return { ok: false, error: "The theme's colors are not readable color values." };
  }
  const type = resolveThemeType(record.type, colors);
  if (type === null) {
    return { ok: false, error: "The theme does not say whether it is dark or light." };
  }
  return {
    ok: true,
    theme: {
      name: normalizeName(record.name),
      type,
      colors,
    },
  };
}

/** Validates a persisted record into an `ImportedThemePreference`, or null. */
export function sanitizeImportedThemePreference(
  value: unknown,
): ImportedThemePreference | null {
  if (!value || typeof value !== "object" || Array.isArray(value)) {
    return null;
  }
  const record = value as Record<string, unknown>;
  const name = typeof record.name === "string" && record.name.trim() ? record.name.trim() : null;
  const type = record.type === "dark" || record.type === "light" ? record.type : null;
  if (!name || !type) {
    return null;
  }
  const colors = normalizeColorValues(record.colors);
  if (!colors || Object.keys(colors).length === 0) {
    return null;
  }
  return { name: name.slice(0, MAX_NAME_LENGTH), type, colors };
}

// ---------------------------------------------------------------------------
// Projection onto semantic tokens
// ---------------------------------------------------------------------------

/**
 * Direct roles a VSCode colors entry carries the same meaning as. Derived
 * roles (borders, hovers, dimmed inks) are projected as `color-mix()` values
 * over the imported background/foreground, so an imported theme stays
 * coherent as one set instead of shipping an underdetermined role table.
 */
const DIRECT_THEME_COLOR_ROLES = [
  ["--color-background", "editor.background"],
  ["--color-surface-editor", "editor.background"],
  ["--color-foreground", "editor.foreground"],
  ["--color-sidebar-background", "sideBar.background"],
  ["--color-sidebar", "sideBar.background"],
  ["--color-sidebar-foreground", "sideBar.foreground"],
  ["--color-input", "input.background"],
  ["--color-text-selection", "editor.selectionBackground"],
  ["--color-text-caret", "editorCursor.foreground"],
  ["--color-ring", "focusBorder"],
  ["--color-primary", "button.background"],
  ["--color-primary-foreground", "button.foreground"],
  ["--color-surface-elevated", "titleBar.activeBackground"],
  ["--color-popover", "editorWidget.background"],
  ["--color-scrollbar-thumb", "scrollbarSlider.background"],
  ["--color-scrollbar-thumb-active", "scrollbarSlider.hoverBackground"],
] as const;

const TERMINAL_THEME_COLOR_ROLES = [
  ["--color-terminal-black", "terminal.ansiBlack"],
  ["--color-terminal-red", "terminal.ansiRed"],
  ["--color-terminal-green", "terminal.ansiGreen"],
  ["--color-terminal-yellow", "terminal.ansiYellow"],
  ["--color-terminal-blue", "terminal.ansiBlue"],
  ["--color-terminal-magenta", "terminal.ansiMagenta"],
  ["--color-terminal-cyan", "terminal.ansiCyan"],
  ["--color-terminal-white", "terminal.ansiWhite"],
  ["--color-terminal-bright-black", "terminal.ansiBrightBlack"],
  ["--color-terminal-bright-red", "terminal.ansiBrightRed"],
  ["--color-terminal-bright-green", "terminal.ansiBrightGreen"],
  ["--color-terminal-bright-yellow", "terminal.ansiBrightYellow"],
  ["--color-terminal-bright-blue", "terminal.ansiBrightBlue"],
  ["--color-terminal-bright-magenta", "terminal.ansiBrightMagenta"],
  ["--color-terminal-bright-cyan", "terminal.ansiBrightCyan"],
  ["--color-terminal-bright-white", "terminal.ansiBrightWhite"],
] as const;

function mix(colorVariable: string, percentage: number, intoVariable: string): string {
  return `color-mix(in srgb, ${colorVariable} ${percentage}%, ${intoVariable})`;
}

/**
 * Sorting the record literal keys keeps the emitted override order stable for
 * tests and for reuse across applications.
 */
function sortedEntryRecord(entries: Array<[string, string]>): Record<string, string> {
  return Object.fromEntries(entries.sort(([a], [b]) => a.localeCompare(b)));
}

export function buildImportedThemeTokenOverrides(
  theme: ImportedThemePreference,
): Record<string, string> {
  const colors = theme.colors;
  const overrides: Array<[string, string]> = [];
  const put = (name: string, value: string) => overrides.push([name, value]);

  for (const [tokenName, themeKey] of DIRECT_THEME_COLOR_ROLES) {
    const value = colors[themeKey];
    if (value) {
      put(tokenName, value);
    }
  }
  for (const [tokenName, themeKey] of TERMINAL_THEME_COLOR_ROLES) {
    const value = colors[themeKey];
    if (value) {
      put(tokenName, value);
    }
  }

  const panelBackground = colors["panel.background"];
  if (panelBackground) {
    put("--color-surface", panelBackground);
  }

  put(
    "--color-faint",
    mix("var(--color-foreground)", 45, "var(--color-background)"),
  );
  put(
    "--color-foreground-tertiary",
    mix("var(--color-foreground)", 55, "var(--color-background)"),
  );
  put(
    "--color-foreground-secondary",
    mix("var(--color-foreground)", 78, "var(--color-background)"),
  );
  put(
    "--color-muted-foreground",
    mix("var(--color-foreground)", 65, "var(--color-background)"),
  );
  put(
    "--color-border",
    mix("var(--color-foreground)", 12, "var(--color-background)"),
  );
  put(
    "--color-border-light",
    mix("var(--color-foreground)", 7, "var(--color-background)"),
  );
  put(
    "--color-border-heavy",
    mix("var(--color-foreground)", 22, "var(--color-background)"),
  );
  put(
    "--color-hover",
    mix("var(--color-foreground)", 6, "var(--color-background)"),
  );
  put(
    "--color-active",
    mix("var(--color-foreground)", 10, "var(--color-background)"),
  );
  put(
    "--color-selected",
    mix("var(--color-foreground)", 14, "var(--color-background)"),
  );

  return sortedEntryRecord(overrides);
}
