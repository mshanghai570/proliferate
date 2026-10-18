import { useRef, useState } from "react";
import { resolveRuntimeCacheScopeKey, useAnyHarnessRuntimeContext } from "@anyharness/sdk-react";
import { useProjectMcpServers } from "#product/hooks/access/anyharness/use-project-mcp-servers";
import { parseProjectMcpServer } from "#product/lib/domain/settings/project-mcp";

export function useProjectMcpEditorIdentity(repo: string) {
  const runtime = useAnyHarnessRuntimeContext();
  return JSON.stringify([resolveRuntimeCacheScopeKey(runtime), runtime.runtimeUrl?.trim() ?? "", repo]);
}

export function useProjectMcpEditor(repo: string) {
  const access = useProjectMcpServers(repo);
  const [name, setName] = useState("");
  const [source, setSource] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const [pending, setPending] = useState(false);
  const busy = useRef(false);
  const [confirm, setConfirm] = useState<{ action: "replace" | "remove"; name: string } | null>(null);

  async function run(action: () => Promise<void>, clearDraft = false) {
    if (busy.current) return;
    busy.current = true;
    setPending(true); setError(null); setNotice(null);
    try {
      await action();
      if (clearDraft) { setName(""); setSource(""); }
      setConfirm(null);
      setNotice("Saved. Changes apply when a session next launches; running sessions keep their current tools.");
    } catch {
      setConfirm(null);
      setError("Could not save MCP settings. Check the configuration, runtime connection, and encryption key, then try again.");
    } finally { busy.current = false; setPending(false); }
  }

  function save(replacing = false) {
    try {
      const server = parseProjectMcpServer(name, source);
      setError(null);
      if (!replacing && access.query.data?.servers.some((server) => server.name === name)) {
        setConfirm({ action: "replace", name });
        return;
      }
      void run(() => access.save(name, server), true);
    } catch (cause) { setError(cause instanceof Error ? cause.message : "Check the server configuration."); }
  }

  return {
    ...access, name, setName, source, setSource, error, notice, pending, confirm, setConfirm, save,
    toggle: (serverName: string, enabled: boolean) => { void run(() => access.toggle(serverName, enabled)); },
    confirmAction: () => {
      if (confirm?.action === "replace") save(true);
      if (confirm?.action === "remove") { const target = confirm.name; void run(() => access.remove(target)); }
    },
  };
}
