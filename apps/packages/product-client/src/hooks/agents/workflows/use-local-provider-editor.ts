import { useRef, useState } from "react";
import type { CreateAgentProviderConfigRequest } from "@proliferate/cloud-sdk";
import { useLocalProviders } from "#product/hooks/access/anyharness/use-local-providers";

export function useLocalProviderEditor() {
  const access = useLocalProviders();
  const busy = useRef(false);
  const [pending, setPending] = useState(false);
  const [open, setOpen] = useState(false);
  const [removeId, setRemoveId] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  async function run(action: () => Promise<void>, closeForm = false) {
    if (busy.current) return;
    busy.current = true; setPending(true); setError(null); setNotice(null);
    try {
      await action();
      if (closeForm) setOpen(false);
      setRemoveId(null);
      setNotice("Saved on this Mac. Changes apply to new agent processes. Refresh Models to check availability.");
    } catch {
      setRemoveId(null);
      void access.query.refetch();
      setError("Could not save local provider settings. Check the runtime connection, encryption key, and managed authentication status, then retry.");
    } finally { busy.current = false; setPending(false); }
  }
  return { ...access, pending, open, removeId, error, notice,
    show: () => { setError(null); setOpen(true); }, close: () => { if (!busy.current) setOpen(false); },
    requestRemove: (id: string | null) => { if (!busy.current) setRemoveId(id); },
    confirmRemove: () => { if (removeId) void run(() => access.remove(removeId)); },
    toggle: (id: string, enabled: boolean) => { void run(() => access.toggle(id, enabled)); },
    submit: (input: CreateAgentProviderConfigRequest) => {
      void run(() => access.save({ name: input.value.name, endpoint: input.value.endpoint,
        apiFormat: input.value.apiFormat ?? "chat", models: input.value.models.split(/\r?\n/u), apiKey: input.value.apiKey ?? null }), true);
    },
  };
}
