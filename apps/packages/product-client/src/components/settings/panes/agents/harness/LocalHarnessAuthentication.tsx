import { useState } from "react";
import { Button } from "#product/primitives/Button";
import { Badge } from "#product/primitives/Badge";
import { SettingsSection } from "#product/primitives/patterns/settings/SettingsSection";
import { CliDetails } from "#product/components/settings/panes/agents/harness/HarnessAuthCliDetails";
import { deriveAuthStatus } from "#product/components/settings/panes/agents/harness/HarnessAuthStatusBadge";
import type { HarnessAuthEditorApi } from "#product/hooks/agents/workflows/use-harness-auth-editor";

export function LocalHarnessAuthentication({ displayName, editor }: {
  displayName: string;
  editor: HarnessAuthEditorApi;
}) {
  const status = deriveAuthStatus("cli", editor);
  const [refreshing, setRefreshing] = useState(false);
  const [error, setError] = useState<string | null>(null);
  async function refresh() {
    setRefreshing(true);
    setError(null);
    try {
      await editor.loginWorkflow.refreshAgentReadiness();
    } catch {
      setError("Could not refresh agent sign-in. Check the local runtime connection and retry.");
    } finally {
      setRefreshing(false);
    }
  }
  return (
    <SettingsSection title="Agent sign-in" titleWeight="emphasized" surface="plain"
      description={`Use ${displayName}'s own login on this Mac. A Proliferate account is not required for local agents.`}>
      <div className="flex flex-wrap items-center gap-3">
        <Badge tone={status.tone}>{status.label}</Badge>
        <Button type="button" variant="ghost" size="sm" disabled={refreshing}
          onClick={() => { void refresh(); }}>{refreshing ? "Checking sign-in…" : "Refresh agent sign-in"}</Button>
      </div>
      <CliDetails editor={editor} />
      {!editor.localAgent?.supportsLogin ? <p className="text-ui-sm text-muted-foreground">Use the agent's documented login on this Mac, then refresh its sign-in status.</p> : null}
      {error ? <p role="alert" className="text-ui-sm text-destructive">{error}</p> : null}
    </SettingsSection>
  );
}
