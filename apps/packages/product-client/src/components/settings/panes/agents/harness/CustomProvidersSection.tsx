import { Button } from "#product/primitives/Button";
import { Switch } from "#product/primitives/Switch";
import { CustomProviderModal } from "#product/components/settings/panes/agents/harness/CustomProviderModal";
import { useCustomProviders } from "#product/hooks/agents/workflows/use-custom-providers";
import type { HarnessAuthEditorApi } from "#product/hooks/agents/workflows/use-harness-auth-editor";

export function CustomProvidersSection({ editor }: { editor: HarnessAuthEditorApi }) {
  const providers = useCustomProviders(editor);
  return (
    <div className="space-y-3">
      {providers.rows.length > 0 ? (
        <ul className="divide-y divide-border">
          {providers.rows.map((row) => (
            <li key={row.uid} className="flex items-center gap-3 py-2">
              <span className="min-w-0 flex-1 truncate text-ui">{row.title}</span>
              <Switch checked={row.enabled} disabled={providers.busy || (!row.enabled && (editor.apiKeyDisallowed || editor.harnessDisallowed))}
                aria-label={`Enable ${row.title}`}
                onChange={(enabled) => editor.handleRowEnabledToggle(row.uid, enabled)} />
              <Button type="button" variant="ghost" size="sm" disabled={providers.busy}
                aria-label={`Remove ${row.title}`} onClick={() => editor.handleRemoveRow(row.uid)}>Remove</Button>
            </li>
          ))}
        </ul>
      ) : null}
      <Button type="button" variant="secondary" size="sm" onClick={providers.show}
        disabled={providers.busy || !editor.authReady || editor.apiKeyDisallowed || editor.harnessDisallowed}>
        Add OpenAI-compatible provider
      </Button>
      {providers.open ? <CustomProviderModal onClose={providers.close} onSubmit={providers.submit}
        busy={providers.busy} error={providers.error} /> : null}
    </div>
  );
}
