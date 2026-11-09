import { useState, type ChangeEvent } from "react";
import { Button } from "#product/primitives/Button";
import { SettingsRow } from "#product/primitives/patterns/settings/SettingsRow";
import { parseImportedAppTheme } from "#product/lib/domain/preferences/imported-app-theme";
import { useUserPreferencesStore } from "#product/stores/preferences/user-preferences-store";

const MAX_THEME_BYTES = 1_048_576;

export function ImportedAppThemePreference() {
  const theme = useUserPreferencesStore((state) => state.importedAppTheme);
  const setMultiple = useUserPreferencesStore((state) => state.setMultiple);
  const setPreference = useUserPreferencesStore((state) => state.set);
  const [error, setError] = useState<string | null>(null);

  const importFile = async (event: ChangeEvent<HTMLInputElement>) => {
    const file = event.currentTarget.files?.[0];
    event.currentTarget.value = "";
    if (!file) return;
    if (file.size > MAX_THEME_BYTES) {
      setError("Choose a JSON theme smaller than 1 MB.");
      return;
    }
    try {
      const importedAppTheme = parseImportedAppTheme(await file.text());
      setMultiple({ importedAppTheme, colorMode: importedAppTheme.type });
      setError(null);
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : "Could not read this theme file.");
    }
  };

  return <SettingsRow
    label="Imported app colors"
    description="Import core background, text, sidebar, and button colors from an editor color-theme JSON file. For syntax colors, choose a supported theme from Extensions → Installed."
  >
    <div className="flex w-72 max-w-full flex-col gap-2">
      <input
        aria-label="Import app theme JSON"
        accept=".json,application/json"
        className="max-w-full text-ui text-foreground file:mr-2 file:rounded-md file:border file:border-border file:bg-card file:px-2 file:py-1 file:text-ui file:text-foreground"
        type="file"
        onChange={(event) => { void importFile(event); }}
      />
      {theme && <div className="flex items-center justify-between gap-2 text-ui-sm text-muted-foreground">
        <span>{theme.name} ({theme.type})</span>
        <Button type="button" variant="secondary" onClick={() => {
          setPreference("importedAppTheme", null);
          setError(null);
        }}>Reset colors</Button>
      </div>}
      {error && <p role="alert" className="text-ui-sm text-destructive">{error}</p>}
    </div>
  </SettingsRow>;
}
