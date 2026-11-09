import { useState } from "react";
import type { ExtensionPackageIdentity } from "@anyharness/sdk";
import { useExtensionSyntaxThemes } from "#product/hooks/access/extensions/use-extension-syntax-themes";
import { importedExtensionColors } from "#product/lib/domain/extensions/extension-color-theme";
import { sanitizeImportedSyntaxTheme, type ImportedSyntaxTheme } from "#product/lib/domain/preferences/imported-syntax-theme";
import type { ImportedAppTheme } from "#product/lib/domain/preferences/imported-app-theme";
import { useUserPreferencesStore } from "#product/stores/preferences/user-preferences-store";

export function useInstalledExtensionSyntax(identity: ExtensionPackageIdentity) {
  const [open, setOpen] = useState(false);
  const [applied, setApplied] = useState<string | null>(null);
  const query = useExtensionSyntaxThemes(identity, open);
  const setMultiple = useUserPreferencesStore((state) => state.setMultiple);
  return { open, setOpen, applied, query,
    themes: query.data?.themes.map((source) => {
      const colors = importedExtensionColors(source);
      const syntax = source.available && colors ? sanitizeImportedSyntaxTheme({ name: source.label, type: source.colorMode,
        background: colors.background, foreground: colors.foreground, rules: source.rules }) : null;
      return { label: source.label, colors, syntax };
    }) ?? [],
    apply: (syntax: ImportedSyntaxTheme, colors: ImportedAppTheme) => {
      setMultiple({ importedSyntaxTheme: syntax, importedAppTheme: colors, colorMode: syntax.type });
      setApplied(syntax.name);
    },
  };
}
