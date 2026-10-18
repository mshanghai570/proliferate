import { useMemo } from "react";
import type { ThemeRegistration } from "shiki";
import { useUserPreferencesStore } from "#product/stores/preferences/user-preferences-store";
import { useResolvedMode } from "#product/hooks/theme/derived/use-resolved-mode";

export function useSyntaxHighlightTheme(): ThemeRegistration | undefined {
  const mode = useResolvedMode();
  const theme = useUserPreferencesStore((state) => state.importedSyntaxTheme);
  return useMemo(() => theme && theme.type === mode ? {
    name: "proliferate-imported-syntax", type: theme.type,
    colors: { "editor.foreground": theme.foreground, "editor.background": theme.background },
    tokenColors: theme.rules.map((rule) => ({ scope: rule.scopes, settings: { foreground: rule.foreground } })),
  } : undefined, [theme, mode]);
}
