import { useCallback, useEffect, useState, type FormEvent } from "react";
import { useProductHost } from "@proliferate/product-client/host/ProductHostProvider";
import { Button } from "#product/primitives/Button";
import { Input } from "#product/primitives/Input";
import { Label } from "#product/primitives/Label";
import { SettingsEmptyState } from "#product/primitives/patterns/settings/SettingsEmptyState";
import { SettingsPageBody } from "#product/primitives/patterns/settings/SettingsPageBody";
import { SettingsSection } from "#product/primitives/patterns/settings/SettingsSection";
import { useProductStorageContext } from "#product/hooks/persistence/facade/use-product-storage-context";
import {
  addNativeProviderProfile,
  discoverNativeProviderModels,
  loadNativeProviderProfiles,
  refreshNativeProviderKeyStatus,
  removeNativeProviderProfileMetadata,
  updateNativeProviderProfile,
  type NativeProviderProfile,
} from "#product/lib/domain/agents/native-provider-profiles";

export function NativeProviderProfilesPane() {
  const host = useProductHost();
  const storageContext = useProductStorageContext();
  const [stableStorageContext] = useState(storageContext);
  const [profiles, setProfiles] = useState<NativeProviderProfile[]>([]);
  const [loading, setLoading] = useState(true);
  const [busyProfileId, setBusyProfileId] = useState<string | null>(null);
  const [name, setName] = useState("");
  const [endpoint, setEndpoint] = useState("");
  const [formError, setFormError] = useState<string | null>(null);
  const [editingProfileId, setEditingProfileId] = useState<string | null>(null);
  const [editName, setEditName] = useState("");
  const [editEndpoint, setEditEndpoint] = useState("");
  const [editError, setEditError] = useState<string | null>(null);
  const native = host.desktop?.aiProviders;
  const supported = native?.isSupported() === true;

  useEffect(() => {
    let active = true;
    setLoading(true);
    void loadNativeProviderProfiles(stableStorageContext).then((stored) => {
      if (active) setProfiles(stored);
    }).finally(() => {
      if (active) setLoading(false);
    });
    return () => { active = false; };
  }, [stableStorageContext]);

  const persistKeyStatus = useCallback(async (current: NativeProviderProfile[], id: string, hasKey: boolean) => {
    const next = await refreshNativeProviderKeyStatus(stableStorageContext, current, id, hasKey);
    setProfiles(next);
  }, [stableStorageContext]);

  async function addProfile(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setFormError(null);
    try {
      const next = await addNativeProviderProfile(stableStorageContext, profiles, { name, endpoint });
      setProfiles(next);
      setName("");
      setEndpoint("");
    } catch (error) {
      setFormError(error instanceof Error ? error.message : "Could not add provider.");
    }
  }

  function beginEdit(profile: NativeProviderProfile) {
    setEditingProfileId(profile.id);
    setEditName(profile.name);
    setEditEndpoint(profile.endpoint);
    setEditError(null);
  }

  async function saveEdit(profile: NativeProviderProfile, event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setEditError(null);
    try {
      const next = await updateNativeProviderProfile(stableStorageContext, profiles, profile.id, {
        name: editName,
        endpoint: editEndpoint,
      });
      setProfiles(next);
      setEditingProfileId(null);
    } catch (error) {
      setEditError(error instanceof Error ? error.message : "Could not update provider.");
    }
  }

  async function promptForKey(profile: NativeProviderProfile) {
    if (!native) return;
    setBusyProfileId(profile.id);
    try {
      const result = await native.promptForKey(profile.id);
      if (result === "saved") await persistKeyStatus(profiles, profile.id, true);
    } catch (error) {
      setProfiles(profiles.map((item) => item.id === profile.id
        ? { ...item, discoveryError: error instanceof Error ? error.message : "Could not save provider key." }
        : item));
    } finally {
      setBusyProfileId(null);
    }
  }

  async function clearKey(profile: NativeProviderProfile) {
    if (!native) return;
    setBusyProfileId(profile.id);
    try {
      await native.deleteKey(profile.id);
      await persistKeyStatus(profiles, profile.id, false);
    } catch (error) {
      setProfiles(profiles.map((item) => item.id === profile.id
        ? { ...item, discoveryError: error instanceof Error ? error.message : "Could not remove provider key." }
        : item));
    } finally {
      setBusyProfileId(null);
    }
  }

  async function discover(profile: NativeProviderProfile) {
    if (!native) return;
    setBusyProfileId(profile.id);
    try {
      const next = await discoverNativeProviderModels(stableStorageContext, profiles, profile.id, native);
      setProfiles(next);
    } finally {
      setBusyProfileId(null);
    }
  }

  async function removeProfile(profile: NativeProviderProfile) {
    if (!native) return;
    setBusyProfileId(profile.id);
    try {
      if (profile.hasKey) await native.deleteKey(profile.id);
      const next = await removeNativeProviderProfileMetadata(stableStorageContext, profiles, profile.id);
      setProfiles(next);
    } catch (error) {
      setProfiles(profiles.map((item) => item.id === profile.id
        ? { ...item, discoveryError: error instanceof Error ? error.message : "Could not remove provider." }
        : item));
    } finally {
      setBusyProfileId(null);
    }
  }

  return (
    <SettingsPageBody data-native-provider-profiles="">
      <header>
        <h1 className="text-title font-semibold tracking-[-0.025em] text-foreground">Native AI providers</h1>
        <p className="mt-1 max-w-2xl text-ui text-muted-foreground">
          Configure OpenAI-compatible endpoints for native Desktop agents. Profiles and discovered model IDs stay on this Mac; API keys are stored in Keychain.
        </p>
        <p className="text-ui-sm text-muted-foreground">
          Only send prompts and workspace context you intend to share with your configured provider.
        </p>
      </header>

      {!supported ? (
        <SettingsEmptyState size="compact" title="Native provider setup is unavailable" description="Open Settings in the packaged Desktop app to manage local provider profiles." />
      ) : (
        <>
          <SettingsSection title="Add provider" description="Use the exact API base URL supplied by your provider, including its version path.">
            <form className="space-y-4 p-4" onSubmit={(event) => { void addProfile(event); }}>
              <div>
                <Label htmlFor="native-provider-name">Provider name</Label>
                <Input id="native-provider-name" value={name} maxLength={100} required autoComplete="off" onChange={(event) => setName(event.target.value)} />
              </div>
              <div>
                <Label htmlFor="native-provider-endpoint">Endpoint URL</Label>
                <Input id="native-provider-endpoint" type="url" value={endpoint} required autoComplete="url" spellCheck={false} placeholder="http://localhost:1234/v1" onChange={(event) => setEndpoint(event.target.value)} />
              </div>
              {formError ? <p role="alert" className="text-ui-sm text-destructive">{formError}</p> : null}
              <Button type="submit">Add provider</Button>
            </form>
          </SettingsSection>

          <SettingsSection title="Provider profiles" description="Keys are entered in a native secure prompt and never sent to the web UI.">
            {loading ? <p className="p-4 text-ui-sm text-muted-foreground">Loading local provider profiles…</p> : profiles.length === 0 ? (
              <p className="p-4 text-ui-sm text-muted-foreground">No native provider profiles yet.</p>
            ) : profiles.map((profile) => {
              const busy = busyProfileId === profile.id;
              return (
                <article key={profile.id} className="border-b border-border p-4 last:border-b-0" data-provider-profile={profile.id}>
                  {editingProfileId === profile.id ? (
                    <form className="mb-4 space-y-3" onSubmit={(event) => { void saveEdit(profile, event); }}>
                      <div><Label htmlFor={`native-provider-name-${profile.id}`}>Provider name</Label><Input id={`native-provider-name-${profile.id}`} value={editName} maxLength={100} required onChange={(event) => setEditName(event.target.value)} /></div>
                      <div><Label htmlFor={`native-provider-endpoint-${profile.id}`}>Endpoint URL</Label><Input id={`native-provider-endpoint-${profile.id}`} type="url" value={editEndpoint} required spellCheck={false} onChange={(event) => setEditEndpoint(event.target.value)} /></div>
                      {editError ? <p role="alert" className="text-ui-sm text-destructive">{editError}</p> : null}
                      <div className="flex gap-2"><Button type="submit">Save changes</Button><Button type="button" variant="ghost" onClick={() => setEditingProfileId(null)}>Cancel</Button></div>
                    </form>
                  ) : (
                    <div className="flex flex-wrap items-start justify-between gap-3">
                    <div className="min-w-0">
                      <h2 className="truncate text-ui font-medium text-foreground">{profile.name}</h2>
                      <p className="break-all font-mono text-ui-sm text-muted-foreground">{profile.endpoint}</p>
                    </div>
                    <div className="flex flex-wrap gap-2">
                      {profile.hasKey ? (
                        <>
                          <span className="self-center text-ui-sm text-muted-foreground">API key saved in Keychain</span>
                          <Button type="button" variant="ghost" disabled={busy} onClick={() => { void promptForKey(profile); }}>Replace key</Button>
                          <Button type="button" variant="ghost" disabled={busy} onClick={() => { void clearKey(profile); }}>Remove key</Button>
                        </>
                      ) : (
                        <Button type="button" variant="secondary" disabled={busy} onClick={() => { void promptForKey(profile); }}>Add API key</Button>
                      )}
                      <Button type="button" variant="secondary" disabled={busy} loading={busy} onClick={() => { void discover(profile); }}>{profile.refreshedAt ? "Refresh models" : "Discover models"}</Button>
                      <Button type="button" variant="ghost" disabled={busy} onClick={() => beginEdit(profile)}>Edit</Button>
                      <Button type="button" variant="ghost" disabled={busy} onClick={() => { void removeProfile(profile); }}>Remove</Button>
                    </div>
                  </div>
                  )}
                  {profile.discoveryError ? <p role="alert" className="mt-3 text-ui-sm text-destructive">{profile.discoveryError}</p> : null}
                  {profile.models.length > 0 ? (
                    <div className="mt-4">
                      <h3 className="text-ui-sm font-medium text-muted-foreground">Discovered models{profile.refreshedAt ? ` · refreshed ${new Date(profile.refreshedAt).toLocaleString()}` : ""}</h3>
                      <ul className="mt-2 space-y-1">{profile.models.map((model) => <li key={model.id} className="font-mono text-ui-sm text-foreground">{model.id}</li>)}</ul>
                    </div>
                  ) : null}
                </article>
              );
            })}
          </SettingsSection>
        </>
      )}
    </SettingsPageBody>
  );
}
