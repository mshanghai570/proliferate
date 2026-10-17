import { describe, expect, it } from "vitest";
import { importSkillFiles } from "./import-skill-files";

function file(path: string, bytes: number[]): File {
  const content = new Uint8Array(bytes);
  return { webkitRelativePath: path, size: content.byteLength, arrayBuffer: async () => content.buffer } as File;
}

describe("skill folder import", () => {
  it("preserves binary resources and relative paths", async () => {
    expect(await importSkillFiles([file("review/SKILL.md", [35, 32, 65]), file("review/resources/icon.bin", [0, 255, 128])]))
      .toEqual({ name: "review", files: [{ path: "SKILL.md", contentBase64: "IyBB" }, { path: "resources/icon.bin", contentBase64: "AP+A" }] });
  });
  it("requires a root SKILL.md", async () => {
    await expect(importSkillFiles([file("review/nested/SKILL.md", [65])])).rejects.toThrow("at its root");
  });
  it("rejects a selection spanning multiple folders", async () => {
    await expect(importSkillFiles([file("review/SKILL.md", [65]), file("other/file", [66])])).rejects.toThrow("one skill folder");
  });
  it("rejects empty and oversized selections before reading", async () => {
    await expect(importSkillFiles([])).rejects.toThrow("up to 128 files");
    await expect(importSkillFiles(Array.from({ length: 129 }, () => file("review/SKILL.md", [65])))).rejects.toThrow("up to 128 files");
    await expect(importSkillFiles([{ size: 1024 * 1024 + 1 } as File])).rejects.toThrow("1 MiB");
  });
});
