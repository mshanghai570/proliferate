import { useEffect, useState } from "react";
import { Button } from "#product/primitives/Button";
import { Input } from "#product/primitives/Input";
import { SettingsRow } from "#product/primitives/patterns/settings/SettingsRow";
import { normalizeLocalFontFamily } from "#product/lib/domain/preferences/local-font-family";
import { useUserPreferencesStore } from "#product/stores/preferences/user-preferences-store";

type FontPreferenceKey = "uiFontFamily" | "codeFontFamily";

export function LocalFontPreference({
  preferenceKey,
  label,
}: {
  preferenceKey: FontPreferenceKey;
  label: string;
}) {
  const selected = useUserPreferencesStore((state) => state[preferenceKey]);
  const setPreference = useUserPreferencesStore((state) => state.set);
  const [draft, setDraft] = useState(selected ?? "");
  const [invalid, setInvalid] = useState(false);

  useEffect(() => { setDraft(selected ?? ""); }, [selected]);

  const commit = () => {
    const normalized = normalizeLocalFontFamily(draft);
    if (draft.trim() && !normalized) {
      setInvalid(true);
      return;
    }
    setPreference(preferenceKey, normalized);
    setDraft(normalized ?? "");
    setInvalid(false);
  };

  return <SettingsRow label={label} description="Enter a font family installed on this Mac. Unavailable fonts use the default.">
    <div className="flex w-60 max-w-full flex-col gap-2">
      <div className="flex items-center gap-2">
        <Input
          aria-label={label}
          aria-invalid={invalid}
          className="min-w-0 flex-1"
          value={draft}
          maxLength={80}
          placeholder="System default"
          onChange={(event) => { setDraft(event.target.value); setInvalid(false); }}
          onBlur={commit}
          onKeyDown={(event) => { if (event.key === "Enter") event.currentTarget.blur(); }}
        />
        <Button type="button" variant="secondary" aria-label={`Reset ${label.toLowerCase()}`} disabled={!selected && !draft} onClick={() => {
          setPreference(preferenceKey, null);
          setDraft("");
          setInvalid(false);
        }}>Reset</Button>
      </div>
      {invalid && <p role="alert" className="text-ui-sm text-destructive">Use letters, numbers, spaces, periods, hyphens, or underscores (up to 80 characters).</p>}
    </div>
  </SettingsRow>;
}
