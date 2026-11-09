import { Button } from "#product/primitives/Button";
import { SettingsSection } from "#product/primitives/patterns/settings/SettingsSection";
import { ConnectServerDialog } from "#product/components/auth/ConnectServerDialog";
import { useConnectServer } from "#product/hooks/auth/workflows/use-connect-server";
import { PROLIFERATE_HOSTED_API_BASE_URL } from "#product/config/capabilities";

export function AccountServerSection() {
  const server = useConnectServer();
  if (!server.available) return null;
  const busy = server.step === "checking" || server.step === "connecting";
  return <SettingsSection title="Account server" description="Choose where this app signs in. Connecting to another server restarts the app.">
    <div className="space-y-3">
      <p className="text-ui-sm text-muted-foreground">Selected server: {server.connectedServerHost ?? "Proliferate hosted service"}</p>
      <div className="flex flex-wrap gap-2">
        {server.connectedServerHost ? <Button type="button" variant="secondary" size="sm" disabled={busy}
          onClick={() => { void server.openForUrl(PROLIFERATE_HOSTED_API_BASE_URL); }}>Use Proliferate hosted service</Button> : null}
        <Button type="button" variant="ghost" size="sm" disabled={busy} onClick={server.open}>Connect to a server</Button>
      </div>
    </div>
    <ConnectServerDialog controller={server} />
  </SettingsSection>;
}
