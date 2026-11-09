import { useState } from "react";
import type { CustomAcpAgent } from "@anyharness/sdk";
import { resolveRuntimeCacheScopeKey, useAnyHarnessRuntimeContext } from "@anyharness/sdk-react";
import { Button } from "#product/primitives/Button";
import { NoticeBanner } from "#product/primitives/patterns/NoticeBanner";
import { ModalShell } from "#product/primitives/patterns/ModalShell";
import { SettingsPageBody } from "#product/primitives/patterns/settings/SettingsPageBody";
import { SettingsSection } from "#product/primitives/patterns/settings/SettingsSection";
import { useCustomAgents } from "#product/hooks/access/anyharness/agents/use-custom-agents";
import { useAgentCatalog } from "#product/hooks/agents/derived/use-agent-catalog";
import { useAgentSurfaceStore } from "#product/stores/ui/agent-surface-store";
import { CustomAgentModal } from "#product/components/settings/panes/agents/custom/CustomAgentModal";

export function CustomAgentsPane() {
  const runtime = useAnyHarnessRuntimeContext();
  return <CustomAgentControls key={`${resolveRuntimeCacheScopeKey(runtime)}:${runtime.runtimeUrl}`} />;
}

function CustomAgentControls() {
  const local = useAgentSurfaceStore((state) => state.surface === "local");
  const access = useCustomAgents(local);
  const { agentsByKind } = useAgentCatalog();
  const [adding, setAdding] = useState(false);
  const [removing, setRemoving] = useState<CustomAcpAgent | null>(null);
  const busy = access.register.isPending || access.remove.isPending || access.refresh.isPending;
  return (
    <SettingsPageBody>
      <h1 className="text-title font-semibold">Custom agents</h1>
      {!local ? <NoticeBanner>Custom ACP agents are configured for local sessions. Select Local to manage them.</NoticeBanner> : (
        <SettingsSection title="ACP agents" description="Connect your own installed agents. Available models and controls are discovered from each agent."
          action={<Button size="sm" onClick={() => { access.register.reset(); setAdding(true); }} disabled={!access.runtimeAvailable || busy}>Connect agent</Button>}>
          {!access.runtimeAvailable ? <p className="p-3 text-ui text-muted-foreground">Connect a local runtime to manage agents.</p> : null}
          {access.query.isLoading ? <p className="p-3 text-ui text-muted-foreground">Loading agents…</p> : null}
          {access.query.isError ? <NoticeBanner tone="destructive" action={<Button size="sm" onClick={() => void access.query.refetch()}>Retry</Button>}>Could not load custom agents.</NoticeBanner> : null}
          {access.query.isSuccess && access.query.data.length === 0 ? <p className="p-3 text-ui text-muted-foreground">No custom agents connected.</p> : null}
          <ul className="divide-y divide-border">
            {(access.query.data ?? []).map((agent) => (
              <li key={agent.id} className="flex flex-wrap items-center gap-3 p-3">
                <div className="min-w-0 flex-1">
                  <div className="text-ui font-medium">{agent.name}</div>
                  <div className="truncate text-ui-sm text-muted-foreground">{agent.command}</div>
                  <div className="text-ui-sm text-muted-foreground">
                    {agentsByKind.get(agent.id)?.readiness === "ready" ? "Ready" : "Check executable and connection"}
                    {agent.envNames.length ? ` · Environment: ${agent.envNames.join(", ")}` : ""}
                  </div>
                </div>
                <Button size="sm" variant="secondary" disabled={busy} onClick={() => access.refresh.mutate(agent.id)}>Refresh models</Button>
                <Button size="sm" variant="ghost" disabled={busy} aria-label={`Remove ${agent.name}`}
                  onClick={() => { access.remove.reset(); setRemoving(agent); }}>Remove</Button>
              </li>
            ))}
          </ul>
          {access.refresh.isError ? <NoticeBanner tone="destructive">Could not refresh the agent. Check its executable and ACP connection.</NoticeBanner> : null}
          {access.refresh.isSuccess ? <p className="p-3 text-ui-sm text-muted-foreground" role="status">Model discovery requested. Available models will appear in the agent picker.</p> : null}
        </SettingsSection>
      )}
      {local && adding ? <CustomAgentModal busy={access.register.isPending} error={access.register.isError ? "Could not connect the agent. Check the configuration and runtime connection." : null}
        onClose={() => setAdding(false)} onSubmit={(request) => access.register.mutate(request, { onSuccess: () => setAdding(false) })} /> : null}
      {local && removing ? <ModalShell open onClose={() => setRemoving(null)} disableClose={access.remove.isPending}
        title={`Remove ${removing.name}?`} description="Running sessions can continue. Existing sessions will need this registration to launch again.">
        {access.remove.isError ? <NoticeBanner tone="destructive">Could not remove the agent. Try again.</NoticeBanner> : null}
        <div className="flex justify-end gap-2">
          <Button variant="ghost" disabled={access.remove.isPending} onClick={() => setRemoving(null)}>Cancel</Button>
          <Button disabled={access.remove.isPending} loading={access.remove.isPending}
            onClick={() => access.remove.mutate(removing.id, { onSuccess: () => setRemoving(null) })}>Remove agent</Button>
        </div>
      </ModalShell> : null}
    </SettingsPageBody>
  );
}
