import { describe, expect, it } from "vitest";
import { localFontStack, normalizeLocalFontFamily } from "#product/lib/domain/preferences/local-font-family";

describe("local font family", () => {
  it("normalizes a macOS family name and builds a fallback stack", () => {
    expect(normalizeLocalFontFamily("  JetBrains   Mono  ")).toBe("JetBrains Mono");
    expect(localFontStack("JetBrains Mono", "monospace")).toBe('"JetBrains Mono", monospace');
  });

  it("rejects CSS syntax and restores the fallback", () => {
    expect(normalizeLocalFontFamily('Bad"; color: red')).toBeNull();
    expect(localFontStack('Bad"; color: red', "monospace")).toBe("monospace");
    expect(normalizeLocalFontFamily(" ")).toBeNull();
  });
});
