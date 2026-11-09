import type { ExtensionColorTheme } from "@anyharness/sdk";
import { parseImportedAppTheme, type ImportedAppTheme } from "#product/lib/domain/preferences/imported-app-theme";

export function importedExtensionColors(theme: ExtensionColorTheme): ImportedAppTheme | null {
  if (!theme.available) return null;
  try {
    return parseImportedAppTheme(JSON.stringify({ name: theme.label, type: theme.colorMode, colors: theme.colors }));
  } catch {
    return null;
  }
}
