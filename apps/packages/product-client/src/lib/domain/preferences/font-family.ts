const MAX_FONT_FAMILY_LENGTH = 128;

/** Family names must survive as one quoted CSS identifier: no quotes,
 * escapes, separators, or control characters. Spaces stay allowed. */
const DISALLOWED_FONT_FAMILY_CHARACTERS = /["'\\;,{\}/]|\p{C}/u;

/** Validates a persisted font family name, or returns null (built-in stack). */
export function sanitizeFontFamilyPreference(value: unknown): string | null {
  if (typeof value !== "string") {
    return null;
  }
  const trimmed = value.trim();
  if (!trimmed || trimmed.length > MAX_FONT_FAMILY_LENGTH) {
    return null;
  }
  if (DISALLOWED_FONT_FAMILY_CHARACTERS.test(trimmed)) {
    return null;
  }
  return trimmed;
}
