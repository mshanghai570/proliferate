import { expect, it } from "vitest";
import { resolveTerminalFontFamily, TERMINAL_FONT_FAMILY } from "#product/lib/domain/terminals/terminal-grid";

it("uses the selected code font for terminal rendering and falls back safely", () => {
  expect(resolveTerminalFontFamily("JetBrains Mono"))
    .toBe(`"JetBrains Mono", ${TERMINAL_FONT_FAMILY}`);
  expect(resolveTerminalFontFamily(null)).toBe(TERMINAL_FONT_FAMILY);
  expect(resolveTerminalFontFamily('Bad"; color: red')).toBe(TERMINAL_FONT_FAMILY);
});
