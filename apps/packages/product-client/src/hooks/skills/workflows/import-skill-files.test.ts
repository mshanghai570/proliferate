import { describe, expect, it } from "vitest";
import { importSkillFile, importSkillFiles } from "./import-skill-files";

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

describe("standalone skill import", () => {
  function standalone(name: string, bytes: number[]) {
    return { ...file("", bytes), name } as File;
  }
  it("accepts lowercase skill.md and preserves instruction bytes under the canonical path", async () => {
    expect(await importSkillFile(standalone("skill.md", [35, 32, 65]), " Review "))
      .toEqual({ name: "Review", files: [{ path: "SKILL.md", contentBase64: "IyBB" }] });
  });
  it("rejects other filenames, empty instructions, and invalid UTF-8", async () => {
    await expect(importSkillFile(standalone("README.md", [65]), "Review")).rejects.toThrow("Choose a SKILL.md");
    await expect(importSkillFile(standalone("SKILL.md", []), "Review")).rejects.toThrow("1–65,536");
    await expect(importSkillFile(standalone("SKILL.md", [255]), "Review")).rejects.toThrow();
  });
  it("checks byte limits and names before reading", async () => {
    const unreadable = { name: "SKILL.md", size: 65_537 } as File;
    await expect(importSkillFile(unreadable, "Review")).rejects.toThrow("1–65,536");
    for (const name of [" ", "bad\nname", "é".repeat(51)]) {
      await expect(importSkillFile(unreadable, name)).rejects.toThrow("skill name");
    }
  });
});
