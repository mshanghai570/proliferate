import { describe, expect, it } from "vitest";
import {
  importedThemeCssProperties,
  parseImportedAppTheme,
  sanitizeImportedAppTheme,
} from "#product/lib/domain/preferences/imported-app-theme";

const source = JSON.stringify({
  name: "Midnight",
  type: "dark",
  colors: {
    "editor.background": "#101820",
    "editor.foreground": "#ffffff",
    "sideBar.background": "#202830",
    "sideBar.foreground": "#eeeeee",
    "button.background": "#ffffff",
    "button.foreground": "#101820",
  },
});

describe("imported app theme", () => {
  it("maps a readable JSON theme to bounded core color roles", () => {
    const theme = parseImportedAppTheme(source);
    expect(theme.name).toBe("Midnight");
    expect(importedThemeCssProperties(theme)).toMatchObject({
      "--color-background": "#101820",
      "--color-sidebar": "#202830",
      "--color-primary": "#FFFFFF",
    });
  });

  it("rejects CSS syntax and unreadable color pairs", () => {
    expect(() => parseImportedAppTheme(source.replace("#ffffff", "red; background: url(file:///tmp/x)"))).toThrow();
    expect(() => parseImportedAppTheme(source.replace("#ffffff", "#111111"))).toThrow();
    expect(() => parseImportedAppTheme(source.replace("#202830", "not a color"))).toThrow();
    expect(sanitizeImportedAppTheme({ ...parseImportedAppTheme(source), background: "var(--bad)" })).toBeNull();
  });

  it("reports strict JSON and mode requirements", () => {
    expect(() => parseImportedAppTheme("// comment\n" + source)).toThrow("without comments");
    expect(() => parseImportedAppTheme(source.replace('"dark"', '"hc"'))).toThrow("dark or light");
  });
});
