import { Button } from "#product/primitives/Button";
import { SettingsRow } from "#product/primitives/patterns/settings/SettingsRow";
import { useUserPreferencesStore } from "#product/stores/preferences/user-preferences-store";

export function ImportedSyntaxThemePreference() {
  const theme = useUserPreferencesStore((state) => state.importedSyntaxTheme);
  const set = useUserPreferencesStore((state) => state.set);
  return <SettingsRow label="Imported syntax colors" description="Choose a supported syntax theme from Extensions → Installed. Its color copy remains available after uninstall. It applies while the app uses the matching light or dark mode.">
    {theme ? <div className="flex flex-wrap items-center gap-2 text-ui-sm">
      <span>{theme.name} ({theme.type})</span>
      <Button variant="secondary" onClick={() => set("importedSyntaxTheme", null)}>Reset syntax colors</Button>
    </div> : <span className="text-ui-sm text-muted-foreground">Built-in syntax colors</span>}
  </SettingsRow>;
}
