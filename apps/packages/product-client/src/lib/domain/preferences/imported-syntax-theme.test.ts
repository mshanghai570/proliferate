import { describe, expect, it } from "vitest";
import { sanitizeImportedSyntaxTheme } from "./imported-syntax-theme";
import { migrateUserPreferences } from "#product/lib/domain/preferences/user/migration";
import { selectPersistedUserPreferencesSlice } from "#product/lib/domain/preferences/persisted-metadata";

const theme = { name: "Night", type: "dark", background: "#101820", foreground: "#ffffff", rules: [{ scopes: ["comment"], foreground: "#aabbcc" }] };
describe("imported syntax theme", () => {
  it("normalizes safe data and preserves it through preference migration and persistence", () => {
    const saved = sanitizeImportedSyntaxTheme(theme)!;
    expect(saved.rules[0].foreground).toBe("#AABBCC");
    const preferences = migrateUserPreferences({ importedSyntaxTheme: saved }).preferences;
    expect(selectPersistedUserPreferencesSlice(preferences).importedSyntaxTheme).toEqual(saved);
    expect(migrateUserPreferences({}).preferences.importedSyntaxTheme).toBeNull();
  });
  it("admits bounded scope groups used by published themes", () => {
    expect(sanitizeImportedSyntaxTheme({ ...theme, rules: [{ scopes: Array(26).fill("meta." + "x".repeat(613)), foreground: "#AABBCC" }] })).not.toBeNull();
  });
  it("rejects unreadable, oversized, and non-data input", () => {
    for (const input of [null, {}, { ...theme, foreground: "#101820" },
      { ...theme, rules: [] }, { ...theme, rules: Array(257).fill(theme.rules[0]) },
      { ...theme, rules: [{ scopes: ["a".repeat(1025)], foreground: "#aabbcc" }] },
      { ...theme, rules: [{ scopes: ["comment"], foreground: "url(secret)" }] },
      { ...theme, rules: [{ scopes: ["comment\n"], foreground: "#aabbcc" }] },
      { ...theme, rules: Array(256).fill({ scopes: ["a".repeat(256)], foreground: "#aabbcc" }) },
    ]) expect(sanitizeImportedSyntaxTheme(input)).toBeNull();
  });
});
