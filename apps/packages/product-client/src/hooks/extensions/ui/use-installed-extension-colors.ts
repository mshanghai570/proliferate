import { useState } from "react";
import type { ExtensionPackageIdentity } from "@anyharness/sdk";
import { useExtensionColorThemes } from "#product/hooks/access/extensions/use-extension-color-themes";
import { importedExtensionColors } from "#product/lib/domain/extensions/extension-color-theme";
import type { ImportedAppTheme } from "#product/lib/domain/preferences/imported-app-theme";
import { useUserPreferencesStore } from "#product/stores/preferences/user-preferences-store";

export function useInstalledExtensionColors(identity: ExtensionPackageIdentity) {
  const [open, setOpen] = useState(false);
  const [applied, setApplied] = useState<string | null>(null);
  const query = useExtensionColorThemes(identity, open);
  const setMultiple = useUserPreferencesStore((state) => state.setMultiple);
  return {
    open, setOpen, applied, query,
    themes: query.data?.themes.map((theme) => ({ label: theme.label, colors: importedExtensionColors(theme) })) ?? [],
    apply: (colors: ImportedAppTheme) => {
      setMultiple({ importedAppTheme: colors, colorMode: colors.type });
      setApplied(colors.name);
    },
  };
}
