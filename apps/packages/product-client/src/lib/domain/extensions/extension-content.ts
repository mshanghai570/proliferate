/** Public registry content must not dispatch IDE commands or local-file URLs. */
export function resolveExtensionContentUrl(value: string | null | undefined, base?: string): string | null {
  if (!value?.trim()) return null;
  try {
    const url = new URL(value, base);
    if (url.protocol !== "https:" || url.username || url.password) return null;
    return url.toString();
  } catch {
    return null;
  }
}
