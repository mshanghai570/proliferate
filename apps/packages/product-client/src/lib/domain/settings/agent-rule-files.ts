export function validateAgentRuleText(text: string): string | null {
  if (new TextEncoder().encode(text).length > 65_536) return "Rules must fit within 65,536 UTF-8 bytes.";
  if (text.includes("\0")) return "Rules cannot contain NUL characters.";
  return null;
}
export async function readAgentRuleFile(file: File): Promise<string> {
  if (!/\.(md|txt)$/i.test(file.name)) throw new Error("Choose a Markdown or text file.");
  if (file.size > 65_536) throw new Error("Choose a file of at most 65,536 bytes.");
  const bytes = await file.arrayBuffer();
  let text: string;
  try { text = new TextDecoder("utf-8", { fatal: true }).decode(bytes); }
  catch { throw new Error("Save the file as UTF-8 text and try again."); }
  const error = validateAgentRuleText(text);
  if (error) throw new Error(error);
  return text;
}
