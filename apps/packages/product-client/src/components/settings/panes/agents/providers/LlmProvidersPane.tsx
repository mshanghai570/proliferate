import { Button } from "#product/primitives/Button";
import { Link } from "react-router-dom";
import { SettingsPageBody } from "#product/primitives/patterns/settings/SettingsPageBody";
import { NoticeBanner } from "#product/primitives/patterns/NoticeBanner";
import { HarnessProvidersSection } from "#product/components/settings/panes/agents/harness/HarnessProvidersSection";
import { HarnessAllModelsSection } from "#product/components/settings/panes/agents/harness/HarnessAllModelsSection";
import { useHarnessAuthEditor } from "#product/hooks/agents/workflows/use-harness-auth-editor";
import { buildSettingsHref } from "#product/lib/domain/settings/navigation";

export function LlmProvidersPane() {
  const editor = useHarnessAuthEditor("opencode", "OpenCode", "local");
  const failed = editor.apiKeysQuery.isError || editor.selectionsQuery.isError;
  const loading = editor.apiKeysQuery.data === undefined || editor.selectionsQuery.data === undefined;
  return (
    <SettingsPageBody>
      <header className="space-y-2">
        <h1 className="text-title font-semibold text-foreground">LLM providers</h1>
        <p className="text-ui text-muted-foreground">
          Connect model APIs with your own keys, or use a model server on your Mac.
        </p>
      </header>
      <NoticeBanner tone="info">
        These connections supply models to local OpenCode sessions. Choose OpenCode
        when creating a chat, then select a model reported by the runtime.
        Other harnesses keep their own provider settings.
      </NoticeBanner>
      <p className="text-ui text-muted-foreground">
        Provider keys use the encrypted credential vault. Saving a provider does
        not test its connection or confirm that its models support coding tools.
        Changes apply to new agent processes.
      </p>
      {editor.deliveryPending ? (
        <p role="status" className="text-ui text-muted-foreground">Applying provider settings to this Mac…</p>
      ) : null}
      {failed ? (
        <NoticeBanner tone="destructive" action={(
          <Button variant="secondary" size="sm" onClick={() => {
            void editor.apiKeysQuery.refetch();
            void editor.selectionsQuery.refetch();
          }}>Retry provider settings</Button>
        )}>
          Could not load saved provider settings. Retry before making changes.
        </NoticeBanner>
      ) : loading ? (
        <p role="status" className="text-ui text-muted-foreground">Loading saved provider settings…</p>
      ) : <HarnessProvidersSection editor={editor} />}
      <HarnessAllModelsSection harnessKind="opencode" displayName="OpenCode" surface="local" />
      <nav aria-label="Provider setup">
        <Link className="text-ui text-foreground underline" to={buildSettingsHref({ section: "agent-opencode" })}>
          Set up the OpenCode runtime
        </Link>
      </nav>
    </SettingsPageBody>
  );
}
