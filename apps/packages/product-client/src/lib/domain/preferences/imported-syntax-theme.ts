import { sanitizeImportedAppTheme } from "#product/lib/domain/preferences/imported-app-theme";

export interface ImportedSyntaxTheme {
  name: string;
  type: "dark" | "light";
  background: string;
  foreground: string;
  rules: { scopes: string[]; foreground: string }[];
}
const color = (value: unknown): value is string => typeof value === "string" && /^#[0-9a-fA-F]{6}$/.test(value);
const record = (value: unknown): Record<string, unknown> | null => value !== null && typeof value === "object" && !Array.isArray(value) ? value as Record<string, unknown> : null;

export function sanitizeImportedSyntaxTheme(value: unknown): ImportedSyntaxTheme | null {
  const input = record(value);
  if (!input || typeof input.name !== "string" || !input.name.trim() || input.name.length > 80
    || (input.type !== "dark" && input.type !== "light") || !color(input.background) || !color(input.foreground)
    || !Array.isArray(input.rules) || input.rules.length === 0 || input.rules.length > 256) return null;
  if (!sanitizeImportedAppTheme({ name: input.name, type: input.type, background: input.background, foreground: input.foreground,
    sidebarBackground: input.background, sidebarForeground: input.foreground, buttonBackground: null, buttonForeground: null })) return null;
  const rules: ImportedSyntaxTheme["rules"] = [];
  let chars = 0;
  const encoder = new TextEncoder();
  for (const value of input.rules) {
    const rule = record(value);
    if (!rule || !color(rule.foreground) || !Array.isArray(rule.scopes) || !rule.scopes.length || rule.scopes.length > 32) return null;
    const scopes: string[] = [];
    for (const scope of rule.scopes) {
      if (typeof scope !== "string" || !scope.trim() || scope.length > 1024 || /[\u0000-\u001f\u007f-\u009f]/u.test(scope)) return null;
      const bytes = encoder.encode(scope).length;
      if (bytes > 1024) return null;
      chars += bytes;
      scopes.push(scope);
    }
    chars += rule.foreground.length;
    if (chars > 32768) return null;
    rules.push({ scopes, foreground: rule.foreground.toUpperCase() });
  }
  return { name: input.name.trim(), type: input.type, background: input.background.toUpperCase(), foreground: input.foreground.toUpperCase(), rules };
}
