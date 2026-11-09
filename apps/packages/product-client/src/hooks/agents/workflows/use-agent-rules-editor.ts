import { useRef, useState } from "react";
import { useAgentRulesAccess } from "#product/hooks/access/agent-rules/use-agent-rules-access";
import { readAgentRuleFile, validateAgentRuleText } from "#product/lib/domain/settings/agent-rule-files";

export function useAgentRulesEditor(kind: string, repositoryId: string | null) {
  const access = useAgentRulesAccess(kind, repositoryId);
  const [draft, setDraft] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [reading, setReading] = useState(false);
  const input = useRef<HTMLInputElement>(null);
  const text = draft ?? access.query.data?.text ?? "";
  const validation = validateAgentRuleText(text);
  const busy = reading || access.mutation.isPending;
  const ready = access.query.isSuccess && !access.query.isError && access.available;
  const dirty = draft !== null;
  const save = async (value: string | null) => {
    setError(null);
    try { await access.mutation.mutateAsync(value); setDraft(null); }
    catch { setError("Could not save rules. Check the runtime connection and retry."); }
  };
  const importFile = async (file: File | undefined) => {
    if (!file) return;
    setReading(true); setError(null);
    try { setDraft(await readAgentRuleFile(file)); }
    catch (cause) { setError(cause instanceof Error ? cause.message : "Could not read the file. Choose it again."); }
    finally { setReading(false); if (input.current) input.current.value = ""; }
  };
  return { access, text, dirty, busy, ready, validation, error, input,
    edit: setDraft, discard: () => { setDraft(null); setError(null); }, save, importFile,
  };
}
