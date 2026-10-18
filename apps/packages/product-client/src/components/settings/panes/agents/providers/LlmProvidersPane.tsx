import { Link } from "react-router-dom";
import { Button } from "#product/primitives/Button";
import { SettingsPageBody } from "#product/primitives/patterns/settings/SettingsPageBody";
import { SettingsSection } from "#product/primitives/patterns/settings/SettingsSection";
import { SettingsRow } from "#product/primitives/patterns/settings/SettingsRow";
import { NoticeBanner } from "#product/primitives/patterns/NoticeBanner";
import { ConfirmationDialog } from "#product/primitives/patterns/ConfirmationDialog";
import { CustomProviderModal } from "#product/components/settings/panes/agents/harness/CustomProviderModal";
import { HarnessAllModelsSection } from "#product/components/settings/panes/agents/harness/HarnessAllModelsSection";
import { useLocalProviders } from "#product/hooks/access/anyharness/use-local-providers";
import { useLocalProviderEditor } from "#product/hooks/agents/workflows/use-local-provider-editor";
import { buildSettingsHref } from "#product/lib/domain/settings/navigation";

export function LlmProvidersPane() {
  const { identity } = useLocalProviders();
  return <LocalProvidersEditor key={identity} />;
}

function LocalProvidersEditor() {
  const editor = useLocalProviderEditor();
  const { query } = editor;
  const blocked = editor.pending || !editor.connected || query.isPending || query.isError;
  const inventory = query.data;
  const providers = inventory?.providers ?? [];
  const canEnable = !!inventory?.encryptionAvailable && !inventory.managedAuthActive;
  const removing = providers.find((row) => row.id === editor.removeId);
  return <SettingsPageBody>
    <header className="space-y-2">
      <h1 className="text-title font-semibold text-foreground">LLM providers</h1>
      <p className="text-ui text-muted-foreground">Connect model APIs or a model server on your Mac. No Proliferate sign-in is required.</p>
    </header>
    <p className="text-ui text-muted-foreground">
      Connections and keys are encrypted on this Mac. Choose OpenCode when creating a chat,
      then select a model reported by the runtime. Changes apply to new processes.
      Saving does not test connectivity or confirm tool support.
    </p>
    {inventory?.managedAuthActive && <NoticeBanner tone="neutral">
      Managed authentication is active. Local connections are paused while that profile is applied.
      Use the managed provider settings to change its connections.
    </NoticeBanner>}
    <SettingsSection title="Local API connections" description="Add an OpenAI-compatible endpoint, Ollama, or LM Studio. Keys cannot be read back here; remove and add a connection again to change its credentials.">
      {!editor.connected ? <p className="text-ui text-muted-foreground">Connect the local runtime to manage providers.</p>
        : query.isPending ? <p role="status" className="text-ui text-muted-foreground">Loading local providers…</p>
        : query.isError ? <NoticeBanner tone="destructive" action={<Button variant="secondary" onClick={() => { void query.refetch(); }}>Retry provider settings</Button>}>Could not load local provider settings. Update or restart the desktop app, then retry.</NoticeBanner>
        : <ul aria-label="Local providers">
          {providers.map((provider) => <li key={provider.id}>
            <SettingsRow label={provider.name} description={`${provider.modelCount} configured models · ${!provider.enabled ? "Disabled" : inventory?.managedAuthActive ? "Paused by managed authentication" : "Enabled for new launches"}`}>
              <Button variant="secondary" disabled={blocked || (!provider.enabled && !canEnable)} aria-label={`${provider.enabled ? "Disable" : "Enable"} ${provider.name}`} onClick={() => editor.toggle(provider.id, !provider.enabled)}>{provider.enabled ? "Disable" : "Enable"}</Button>
              <Button variant="ghost" disabled={blocked} aria-label={`Remove ${provider.name}`} onClick={() => editor.requestRemove(provider.id)}>Remove</Button>
            </SettingsRow>
          </li>)}
          {providers.length === 0 && <li className="text-ui text-muted-foreground">No local providers configured.</li>}
        </ul>}
      {inventory && !inventory.encryptionAvailable && <NoticeBanner tone="destructive">The runtime encryption key is unavailable. Restart the desktop app before adding a provider.</NoticeBanner>}
      {editor.error && !editor.open && <NoticeBanner tone="destructive">{editor.error}</NoticeBanner>}
      {editor.notice && <p role="status" className="text-ui text-muted-foreground">{editor.notice}</p>}
      <Button variant="secondary" disabled={blocked || !canEnable} onClick={editor.show}>Add API provider</Button>
    </SettingsSection>
    <HarnessAllModelsSection harnessKind="opencode" displayName="OpenCode" surface="local" />
    <nav aria-label="Provider setup" className="flex flex-col gap-2">
      <Link className="text-ui text-foreground underline" to={buildSettingsHref({ section: "agent-opencode" })}>Runtime setup and managed provider settings</Link>
    </nav>
    {editor.open && <CustomProviderModal busy={editor.pending} error={editor.error} onClose={editor.close} onSubmit={editor.submit} />}
    <ConfirmationDialog open={editor.removeId !== null} title={`Remove ${removing?.name ?? "provider"}?`}
      description="Remove this connection and its encrypted credentials from this Mac. Running sessions keep their current credentials until their process stops."
      confirmLabel="Remove provider" confirmVariant="destructive" loading={editor.pending} disableClose={editor.pending}
      onClose={() => editor.requestRemove(null)} onConfirm={editor.confirmRemove} />
  </SettingsPageBody>;
}
