import { useState } from "react";
import type { CreateAgentProviderConfigRequest } from "@proliferate/cloud-sdk";
import { useCustomProviderAccess } from "#product/hooks/access/cloud/use-custom-provider-access";
import type { HarnessAuthEditorApi } from "#product/hooks/agents/workflows/use-harness-auth-editor";

export function useCustomProviders(editor: HarnessAuthEditorApi) {
  const access = useCustomProviderAccess();
  const [open, setOpen] = useState(false);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const keys = new Map((editor.apiKeysQuery.data ?? []).map((key) => [key.id, key]));
  const rows = editor.editorState.rows.flatMap((row) => {
    const key = row.apiKeyId === null ? undefined : keys.get(row.apiKeyId);
    return row.envVarName === null && key?.kind === "openai_compatible"
      ? [{ ...row, title: key.title }] : [];
  });

  function submit(input: CreateAgentProviderConfigRequest) {
    if (saving || editor.busy) return;
    setError(null);
    setSaving(true);
    access.create.mutate(input, {
      onSuccess: (created) => {
        editor.addBoundApiKey(null, "openai_compatible", created.id, {
          onSuccess: () => { setSaving(false); setOpen(false); },
          onError: (message) => {
            access.revoke.mutate(created.id, {
              onSuccess: () => { setError(message); setSaving(false); },
              onError: () => {
                setError(`${message} The saved configuration could not be removed. Reopen Settings before retrying.`);
                setSaving(false);
              },
            });
          },
        });
      },
      onError: (error) => { setSaving(false); setError(error.message); },
    });
  }

  return { rows, open, error, busy: saving || editor.busy,
    show: () => { setError(null); setOpen(true); },
    close: () => { if (!saving) setOpen(false); }, submit,
  };
}
