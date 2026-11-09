import type { ImportSkillRequest } from "@anyharness/sdk";

export async function importSkillFiles(files: File[]): Promise<ImportSkillRequest> {
  if (!files.length || files.length > 128 || files.reduce((total, file) => total + file.size, 0) > 1024 * 1024) {
    throw new Error("Choose a skill folder with up to 128 files and 1 MiB of content.");
  }
  const root = files[0].webkitRelativePath.split("/")[0];
  const entries = await Promise.all(files.map(async (file) => {
    const relative = file.webkitRelativePath;
    if (!relative || relative.split("/")[0] !== root) throw new Error("Choose one skill folder.");
    const bytes = new Uint8Array(await file.arrayBuffer());
    let binary = "";
    for (const byte of bytes) binary += String.fromCharCode(byte);
    return { path: relative.slice(root.length + 1), contentBase64: btoa(binary) };
  }));
  if (!entries.some((entry) => entry.path === "SKILL.md")) throw new Error("Choose a folder containing SKILL.md at its root.");
  return { name: root, files: entries };
}
