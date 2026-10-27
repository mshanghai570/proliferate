import { describe, expect, it } from "vitest";
import {
  buildImportedThemeTokenOverrides,
  parseVscodeThemeText,
  sanitizeImportedThemePreference,
} from "#product/lib/domain/preferences/vscode-theme";

const DARK_PLUS_EXCERPT = {
  name: "Dark+ excerpt",
  type: "dark",
  colors: {
    "editor.background": "#1e1e1e",
    "editor.foreground": "#d4d4d4",
    "sideBar.background": "#252526",
    "sideBar.foreground": "#cccccc",
    "input.background": "#3c3c3c",
    "editor.selectionBackground": "#264f78",
    "editorCursor.foreground": "#aeafad",
    "focusBorder": "#007fd4",
    "button.background": "#0e639c",
    "button.foreground": "#ffffff",
    "panel.background": "#1e1e1e",
    "editorWidget.background": "#252526",
    "titleBar.activeBackground": "#3c3c3c",
    "scrollbarSlider.background": "#79797966",
    "scrollbarSlider.hoverBackground": "#646464b3",
    "terminal.ansiBrightCyan": "rgb(41, 184, 219)",
    "editorGroupHeader.noTabsBackground": "Not A Color",
  },
};

describe("parseVscodeThemeText", () => {
  it("parses the object shape and keeps validated colors", () => {
    const result = parseVscodeThemeText(JSON.stringify(DARK_PLUS_EXCERPT));
    expect(result.ok).toBe(true);
    if (!result.ok) return;
    expect(result.theme.name).toBe("Dark+ excerpt");
    expect(result.theme.type).toBe("dark");
    expect(result.theme.colors["editor.background"]).toBe("#1e1e1e");
    expect(result.theme.colors["terminal.ansiBrightCyan"]).toBe("#29b8db");
    expect(result.theme.colors["editorGroupHeader.noTabsBackground"]).toBeUndefined();
  });

  it("normalizes shorthand hex to six digits", () => {
    const result = parseVscodeThemeText(JSON.stringify({
      name: "Short",
      type: "dark",
      colors: { "editor.background": "#abc" },
    }));
    expect(result.ok).toBe(true);
    if (!result.ok) return;
    expect(result.theme.colors["editor.background"]).toBe("#aabbcc");
  });

  it("infers the mode from editor background luminance when type is absent", () => {
    const dark = parseVscodeThemeText(JSON.stringify({
      colors: { "editor.background": "#181818" },
    }));
    const light = parseVscodeThemeText(JSON.stringify({
      colors: { "editor.background": "#ffffff" },
    }));
    expect(dark.ok && dark.theme.type).toBe("dark");
    expect(light.ok && light.theme.type).toBe("light");
  });

  it("rejects invalid JSON, arrays, missing colors, and unreadable modes", () => {
    expect(parseVscodeThemeText("not json").ok).toBe(false);
    expect(parseVscodeThemeText("[1,2]").ok).toBe(false);
    expect(parseVscodeThemeText("{}").ok).toBe(false);
    expect(parseVscodeThemeText(JSON.stringify({ name: "x", type: "dark" })).ok).toBe(false);
    // A colors object and no type stays parseable: the mode is inferred.
    expect(parseVscodeThemeText(JSON.stringify({ name: "x", colors: { "editor.background": "#ffffff" } })).ok).toBe(true);
  });

  it("caps bomb entries", () => {
    const colors: Record<string, string> = {};
    for (let index = 0; index < 300; index += 1) {
      colors[`key${index}`] = "#010203";
    }
    expect(parseVscodeThemeText(JSON.stringify({ name: "x", type: "dark", colors })).ok).toBe(false);
  });
});

describe("sanitizeImportedThemePreference", () => {
  it("keeps a valid record", () => {
    const theme = sanitizeImportedThemePreference(DARK_PLUS_EXCERPT);
    expect(theme?.name).toBe("Dark+ excerpt");
    expect(theme?.type).toBe("dark");
    expect(theme?.colors["editor.background"]).toBe("#1e1e1e");
  });

  it("returns null for malformed records", () => {
    expect(sanitizeImportedThemePreference(null)).toBeNull();
    expect(sanitizeImportedThemePreference("theme")).toBeNull();
    expect(sanitizeImportedThemePreference({ name: "x", type: "dark" })).toBeNull();
    expect(sanitizeImportedThemePreference({ name: "", type: "dark", colors: { a: "#000000" } })).toBeNull();
  });
});

describe("buildImportedThemeTokenOverrides", () => {
  it("maps the shell colors onto the semantic vocabulary", () => {
    const parse = parseVscodeThemeText(JSON.stringify(DARK_PLUS_EXCERPT));
    if (!parse.ok) throw new Error("parse fixture failed");
    const overrides = buildImportedThemeTokenOverrides(parse.theme);

    expect(overrides["--color-background"]).toBe("#1e1e1e");
    expect(overrides["--color-sidebar"]).toBe("#252526");
    expect(overrides["--color-sidebar-foreground"]).toBe("#cccccc");
    expect(overrides["--color-input"]).toBe("#3c3c3c");
    expect(overrides["--color-text-selection"]).toBe("#264f78");
    expect(overrides["--color-primary"]).toBe("#0e639c");
    expect(overrides["--color-scrollbar-thumb"]).toBe("#79797966");
    expect(overrides["--color-terminal-bright-cyan"]).toBe("#29b8db");
  });

  it("projects derived roles as color-mix values over the imported colors", () => {
    const parse = parseVscodeThemeText(JSON.stringify(DARK_PLUS_EXCERPT));
    if (!parse.ok) throw new Error("parse fixture failed");
    const overrides = buildImportedThemeTokenOverrides(parse.theme);

    expect(overrides["--color-border"]).toBe(
      "color-mix(in srgb, var(--color-foreground) 12%, var(--color-background))",
    );
    expect(overrides["--color-hover"]).not.toContain("#");
  });

  it("stores every emitted token name in the semantic vocabulary", () => {
    const parse = parseVscodeThemeText(JSON.stringify(DARK_PLUS_EXCERPT));
    if (!parse.ok) throw new Error("parse fixture failed");
    const overrides = buildImportedThemeTokenOverrides(parse.theme);
    for (const name of Object.keys(overrides)) {
      expect(name.startsWith("--color-")).toBe(true);
    }
  });
});
