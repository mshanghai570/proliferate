/** A family name resolved by the macOS font system through CSS font-family. */
export function normalizeLocalFontFamily(value: unknown): string | null {
  if (typeof value !== "string") return null;
  const name = value.trim().replace(/\s+/gu, " ");
  if (!name || name.length > 80 || !/^[\p{L}\p{N} ._-]+$/u.test(name)) return null;
  return name;
}

export function localFontStack(name: string | null, fallback: string): string {
  const family = normalizeLocalFontFamily(name);
  return family ? `"${family}", ${fallback}` : fallback;
}
