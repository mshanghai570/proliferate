import { expect, it } from "vitest";
import { readAgentRuleFile, validateAgentRuleText } from "./agent-rule-files";
it("bounds UTF-8 bytes, allows an explicit empty override and rejects NUL", () => {
  expect(validateAgentRuleText("")).toBeNull();
  expect(validateAgentRuleText("é".repeat(32_768))).toBeNull();
  expect(validateAgentRuleText("é".repeat(32_769))).toContain("65,536");
  expect(validateAgentRuleText("a\0b")).toContain("NUL");
});
it("imports exact Markdown and rejects malformed UTF-8 and oversized files", async () => {
  const text = "# Agent\n  preserve whitespace\n";
  const file = { name: "codex.md", size: text.length, arrayBuffer: async () => new TextEncoder().encode(text).buffer } as File;
  expect(await readAgentRuleFile(file)).toBe(text);
  await expect(readAgentRuleFile({ ...file, name: "rules.exe" })).rejects.toThrow("Markdown");
  await expect(readAgentRuleFile({ ...file, size: 65_537 })).rejects.toThrow("65,536");
  await expect(readAgentRuleFile({ ...file, arrayBuffer: async () => new Uint8Array([255]).buffer })).rejects.toThrow("UTF-8");
});
