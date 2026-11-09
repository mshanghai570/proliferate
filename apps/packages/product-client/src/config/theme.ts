import {
  buildUiTextScaleCssVariables,
  DEFAULT_APPEARANCE_SIZE_ID,
  DEFAULT_WINDOW_ZOOM_ID,
  resolveAppearanceSizeId,
  resolveReadableCodeFontScale,
  resolveUiFontScale,
  resolveWindowZoomId,
  resolveWindowZoomScale,
  type ReadableCodeFontSizeId,
  type UiFontSizeId,
  type WindowZoomId,
} from "#product/lib/domain/preferences/appearance";
import {
  buildImportedThemeTokenOverrides,
  sanitizeImportedThemePreference,
  type ImportedThemePreference,
} from "#product/lib/domain/preferences/vscode-theme";
import { sanitizeFontFamilyPreference } from "#product/lib/domain/preferences/font-family";

// ---------------------------------------------------------------------------
// Color mode (the app ships a single Mono theme; mode is the only switch)
// ---------------------------------------------------------------------------

export const COLOR_MODES = ["dark", "light", "system"] as const;
export type ColorMode = (typeof COLOR_MODES)[number];

// ---------------------------------------------------------------------------
// Resolve "system" to actual dark/light
// ---------------------------------------------------------------------------

function resolveMode(mode: ColorMode): "dark" | "light" {
  if (mode === "system") {
    return window.matchMedia("(prefers-color-scheme: dark)").matches
      ? "dark"
      : "light";
  }
  return mode;
}

function applyMode(mode: ColorMode) {
  const resolved = resolveMode(mode);
  document.documentElement.dataset.mode = resolved;
  document.documentElement.style.colorScheme = resolved;
}

export interface AppearancePreference {
  colorMode: ColorMode;
  importedTheme: ImportedThemePreference | null;
  uiFontFamily: string | null;
  codeFontFamily: string | null;
  uiFontSizeId: UiFontSizeId;
  readableCodeFontSizeId: ReadableCodeFontSizeId;
  windowZoomId: WindowZoomId;
}

/**
 * Imported themes ride on the same inline-style layer the text-scale system
 * uses: overriding tokens declaratively would require a generator-level
 * alternate root, which the token authority forbids. The names most recently
 * applied are tracked so switching to mono (or to the other posture's mode)
 * removes exactly what was set.
 */
let appliedImportedThemePropertyNames: string[] = [];

function applyImportedTheme(
  importedThemeRaw: unknown,
  resolvedMode: "dark" | "light",
  root: HTMLElement,
) {
  const theme = importedThemeRaw ? sanitizeImportedThemePreference(importedThemeRaw) : null;
  const overrides =
    theme && theme.type === resolvedMode
      ? buildImportedThemeTokenOverrides(theme)
      : undefined;
  for (const name of appliedImportedThemePropertyNames) {
    root.style.removeProperty(name);
  }
  appliedImportedThemePropertyNames = overrides ? Object.keys(overrides) : [];
  if (overrides) {
    for (const [name, value] of Object.entries(overrides)) {
      root.style.setProperty(name, value);
    }
  }
}

/**
 * Custom fonts prepend one quoted family to the generated font stack. This
 * module is the only inline writer of `--font-sans`/`--font-mono`, so the
 * pristine generated stacks are captured the first time they are needed while
 * no inline override exists — the fallback chain is never restated
 * second-hand and can never drift from the token authority.
 */
let pristineSansFontStack: string | null = null;
let pristineMonoFontStack: string | null = null;

function applyCustomFontFamily(
  root: HTMLElement,
  fontTokenName: string,
  rawFamily: unknown,
  capturedStack: string | null,
): {
  applied: boolean;
  capturedStack: string | null;
} {
  const family = sanitizeFontFamilyPreference(rawFamily);
  if (!family) {
    root.style.removeProperty(fontTokenName);
    return { applied: false, capturedStack };
  }
  let stack = capturedStack;
  if (!stack) {
    if (root.style.getPropertyValue(fontTokenName).trim()) {
      // The pristine value is gone; skip rather than restate a fallback.
      return { applied: false, capturedStack: null };
    }
    stack = getComputedStyle(root).getPropertyValue(fontTokenName).trim() || null;
  }
  if (stack) {
    root.style.setProperty(fontTokenName, `"${family}", ${stack}`);
  }
  return { applied: true, capturedStack: stack };
}

function applyCustomFonts(
  root: HTMLElement,
  input: { uiFontFamily: unknown; codeFontFamily: unknown },
) {
  const sans = applyCustomFontFamily(root, "--font-sans", input.uiFontFamily, pristineSansFontStack);
  pristineSansFontStack = sans.capturedStack;
  const mono = applyCustomFontFamily(root, "--font-mono", input.codeFontFamily, pristineMonoFontStack);
  pristineMonoFontStack = mono.capturedStack;
}

export function applyAppearancePreference({
  colorMode,
  importedTheme,
  uiFontFamily,
  codeFontFamily,
  uiFontSizeId,
  readableCodeFontSizeId,
  windowZoomId,
}: AppearancePreference) {
  const root = document.documentElement;
  const resolvedUiFontSizeId = resolveAppearanceSizeId(uiFontSizeId);
  const resolvedReadableCodeFontSizeId = resolveAppearanceSizeId(readableCodeFontSizeId);
  const resolvedWindowZoomId = resolveWindowZoomId(windowZoomId);
  const uiScale = resolveUiFontScale(resolvedUiFontSizeId);
  const readableCodeScale = resolveReadableCodeFontScale(
    resolvedReadableCodeFontSizeId,
    resolvedUiFontSizeId,
  );
  const windowZoomScale = resolveWindowZoomScale(resolvedWindowZoomId);

  applyCustomFonts(root, { uiFontFamily, codeFontFamily });

  root.dataset.uiFontSize = resolvedUiFontSizeId;
  root.dataset.readableCodeFontSize = resolvedReadableCodeFontSizeId;
  root.dataset.windowZoom = resolvedWindowZoomId;
  applyMode(colorMode);
  applyImportedTheme(importedTheme, resolveMode(colorMode), root);

  for (const [property, value] of Object.entries(buildUiTextScaleCssVariables(uiScale))) {
    root.style.setProperty(property, value);
  }
  root.style.setProperty("--diffs-font-size", readableCodeScale.diffsFontSize);
  root.style.setProperty("--diffs-line-height", readableCodeScale.diffsLineHeight);
  root.style.setProperty("--readable-code-font-size", readableCodeScale.codeFontSize);
  root.style.setProperty("--readable-code-line-height", readableCodeScale.codeLineHeight);
  root.style.setProperty("--proliferate-window-zoom", windowZoomScale.cssValue);

  notify();
}

export function initializeTheme(mode: ColorMode = "dark") {
  applyMode(mode);
  document.documentElement.dataset.uiFontSize = DEFAULT_APPEARANCE_SIZE_ID;
  document.documentElement.dataset.readableCodeFontSize = DEFAULT_APPEARANCE_SIZE_ID;
  document.documentElement.dataset.windowZoom = DEFAULT_WINDOW_ZOOM_ID;
}

// ---------------------------------------------------------------------------
// Subscriber notification
// ---------------------------------------------------------------------------

const listeners = new Set<() => void>();

function notify() {
  listeners.forEach((fn) => fn());
}

export function subscribe(cb: () => void) {
  listeners.add(cb);

  const observer = new MutationObserver(notify);
  observer.observe(document.documentElement, {
    attributes: true,
    attributeFilter: ["data-mode"],
  });

  return () => {
    listeners.delete(cb);
    observer.disconnect();
  };
}

// ---------------------------------------------------------------------------
// Mode change observer (for non-React consumers like xterm)
// ---------------------------------------------------------------------------

export function onThemeChange(callback: () => void): () => void {
  const observer = new MutationObserver(callback);
  observer.observe(document.documentElement, {
    attributes: true,
    attributeFilter: ["data-mode"],
  });
  return () => observer.disconnect();
}

// ---------------------------------------------------------------------------
// Terminal theme reader
// ---------------------------------------------------------------------------

export function getResolvedMode(): "dark" | "light" {
  if (typeof document === "undefined") {
    // Theme-aware consumers can be imported in non-DOM test contexts.
    return "dark";
  }
  return document.documentElement.dataset.mode === "light" ? "light" : "dark";
}

export function getTerminalTheme(): Record<string, string> {
  const s = getComputedStyle(document.documentElement);
  const v = (name: string) => s.getPropertyValue(name).trim();
  return {
    background: v("--color-sidebar"),
    foreground: v("--color-foreground"),
    cursor: v("--color-text-caret") || v("--color-foreground"),
    selectionBackground: v("--color-text-selection") || v("--color-input"),
    black: v("--color-terminal-black"),
    red: v("--color-terminal-red"),
    green: v("--color-terminal-green"),
    yellow: v("--color-terminal-yellow"),
    blue: v("--color-terminal-blue"),
    magenta: v("--color-terminal-magenta"),
    cyan: v("--color-terminal-cyan"),
    white: v("--color-terminal-white"),
    brightBlack: v("--color-terminal-bright-black"),
    brightRed: v("--color-terminal-bright-red"),
    brightGreen: v("--color-terminal-bright-green"),
    brightYellow: v("--color-terminal-bright-yellow"),
    brightBlue: v("--color-terminal-bright-blue"),
    brightMagenta: v("--color-terminal-bright-magenta"),
    brightCyan: v("--color-terminal-bright-cyan"),
    brightWhite: v("--color-terminal-bright-white"),
  };
}
