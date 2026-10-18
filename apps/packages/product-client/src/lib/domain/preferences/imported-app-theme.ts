/** A bounded editor color-theme JSON subset used for core app surfaces. */
export interface ImportedAppTheme {
  name: string;
  type: "dark" | "light";
  background: string;
  foreground: string;
  sidebarBackground: string;
  sidebarForeground: string;
  buttonBackground: string | null;
  buttonForeground: string | null;
}

const HEX_COLOR = /^#[0-9a-fA-F]{6}$/;

function record(value: unknown): Record<string, unknown> | null {
  return value && typeof value === "object" && !Array.isArray(value)
    ? value as Record<string, unknown>
    : null;
}

function color(value: unknown): string | null {
  return typeof value === "string" && HEX_COLOR.test(value) ? value.toUpperCase() : null;
}

function contrast(first: string, second: string): number {
  const luminance = (hex: string) => {
    const channels = [1, 3, 5].map((index) => Number.parseInt(hex.slice(index, index + 2), 16) / 255);
    const [red, green, blue] = channels.map((channel) => channel <= 0.04045
      ? channel / 12.92
      : ((channel + 0.055) / 1.055) ** 2.4);
    return 0.2126 * red + 0.7152 * green + 0.0722 * blue;
  };
  const [lighter, darker] = [luminance(first), luminance(second)].sort((a, b) => b - a);
  return (lighter + 0.05) / (darker + 0.05);
}

export function sanitizeImportedAppTheme(value: unknown): ImportedAppTheme | null {
  const input = record(value);
  if (!input) return null;
  const name = typeof input.name === "string" ? input.name.trim() : "";
  if (!name || name.length > 80 || (input.type !== "dark" && input.type !== "light")) return null;
  const background = color(input.background);
  const foreground = color(input.foreground);
  const sidebarBackground = color(input.sidebarBackground);
  const sidebarForeground = color(input.sidebarForeground);
  const buttonBackground = input.buttonBackground == null ? null : color(input.buttonBackground);
  const buttonForeground = input.buttonForeground == null ? null : color(input.buttonForeground);
  if (!background || !foreground || !sidebarBackground || !sidebarForeground
    || (buttonBackground === null) !== (buttonForeground === null)) return null;
  if (contrast(background, foreground) < 4.5
    || contrast(sidebarBackground, sidebarForeground) < 4.5
    || (buttonBackground && buttonForeground && contrast(buttonBackground, buttonForeground) < 4.5)) return null;
  return { name, type: input.type, background, foreground, sidebarBackground,
    sidebarForeground, buttonBackground, buttonForeground };
}

export function parseImportedAppTheme(source: string): ImportedAppTheme {
  let input: unknown;
  try {
    input = JSON.parse(source);
  } catch {
    throw new Error("Use a JSON theme file without comments or trailing commas.");
  }
  const theme = record(input);
  const colors = record(theme?.colors);
  const background = color(colors?.["editor.background"]);
  const foreground = color(colors?.["editor.foreground"]);
  const sidebarBackground = colors && "sideBar.background" in colors
    ? color(colors["sideBar.background"]) : background;
  const sidebarForeground = colors && "sideBar.foreground" in colors
    ? color(colors["sideBar.foreground"]) : foreground;
  const buttonBackground = color(colors?.["button.background"]);
  const buttonForeground = color(colors?.["button.foreground"]);
  const imported = sanitizeImportedAppTheme({
    name: typeof theme?.name === "string" ? theme.name : "Imported theme",
    type: theme?.type,
    background, foreground, sidebarBackground, sidebarForeground,
    buttonBackground, buttonForeground,
  });
  if (!imported) {
    throw new Error("Theme needs a dark or light type, readable editor colors, and readable optional sidebar and button colors.");
  }
  return imported;
}

export const IMPORTED_THEME_CSS_PROPERTIES = [
  "--color-background", "--color-foreground", "--color-card", "--color-card-foreground",
  "--color-surface", "--color-surface-editor", "--color-sidebar",
  "--color-sidebar-background", "--color-sidebar-foreground",
  "--color-primary", "--color-primary-foreground",
] as const;

export function importedThemeCssProperties(theme: ImportedAppTheme): Record<string, string> {
  return {
    "--color-background": theme.background,
    "--color-foreground": theme.foreground,
    "--color-card": theme.background,
    "--color-card-foreground": theme.foreground,
    "--color-surface": theme.background,
    "--color-surface-editor": theme.background,
    "--color-sidebar": theme.sidebarBackground,
    "--color-sidebar-background": theme.sidebarBackground,
    "--color-sidebar-foreground": theme.sidebarForeground,
    ...(theme.buttonBackground && theme.buttonForeground ? {
      "--color-primary": theme.buttonBackground,
      "--color-primary-foreground": theme.buttonForeground,
    } : {}),
  };
}
