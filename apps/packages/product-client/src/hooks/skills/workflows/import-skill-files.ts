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

export async function importSkillFile(file: File, name: string): Promise<ImportSkillRequest> {
  if (file.name.toLowerCase() !== "skill.md") throw new Error("Choose a SKILL.md file.");
  const label = name.trim();
  if (!label || new TextEncoder().encode(label).length > 100 || /[\u0000-\u001f\u007f-\u009f]/.test(label)) {
    throw new Error("Enter a skill name with 1–100 printable UTF-8 bytes.");
  }
  if (file.size === 0 || file.size > 65_536) throw new Error("Choose a SKILL.md file containing 1–65,536 bytes.");
  const bytes = new Uint8Array(await file.arrayBuffer());
  try {
    new TextDecoder("utf-8", { fatal: true }).decode(bytes);
  } catch {
    throw new Error("Save SKILL.md as UTF-8 text and try again.");
  }
  let binary = "";
  for (const byte of bytes) binary += String.fromCharCode(byte);
  return { name: label, files: [{ path: "SKILL.md", contentBase64: btoa(binary) }] };
}
