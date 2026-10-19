import { useEffect, useState } from "react";
import { resolveRuntimeCacheScopeKey, useAnyHarnessRuntimeContext, useAgentsQuery, useRepoRootsQuery } from "@anyharness/sdk-react";
import { Button } from "#product/primitives/Button";
import { Select } from "#product/primitives/Select";
import { Textarea } from "#product/primitives/Textarea";
import { NoticeBanner } from "#product/primitives/patterns/NoticeBanner";
import { SettingsPageBody } from "#product/primitives/patterns/settings/SettingsPageBody";
import { useAgentSurfaceStore } from "#product/stores/ui/agent-surface-store";
import { useAgentRulesEditor } from "#product/hooks/agents/workflows/use-agent-rules-editor";

export function AgentRulesPane() {
  const runtime = useAnyHarnessRuntimeContext();
  const local = useAgentSurfaceStore((state) => state.surface === "local");
  return <SettingsPageBody><h1 className="text-title font-semibold">Agent rules</h1>
    {local ? <RuleScope key={`${resolveRuntimeCacheScopeKey(runtime)}:${runtime.runtimeUrl}`} /> : <NoticeBanner>Select Local to manage agent rules.</NoticeBanner>}
  </SettingsPageBody>;
}
function RuleScope() {
  const agents = useAgentsQuery();
  const repositories = useRepoRootsQuery();
  const [kind, setKind] = useState("");
  const [repositoryId, setRepositoryId] = useState("");
  const firstKind = agents.data?.[0]?.kind ?? "";
  useEffect(() => { if (!kind && firstKind) setKind(firstKind); }, [kind, firstKind]);
  const selected = kind || firstKind;
  return <RuleEditor key={`${selected}:${repositoryId}`} kind={selected} repositoryId={repositoryId || null}
    agents={agents.data ?? []} repositories={repositories.data ?? []} catalogError={agents.isError || repositories.isError}
    selectAgent={setKind} selectRepository={setRepositoryId} />;
}
type RuleEditorProps = {
  kind: string; repositoryId: string | null;
  agents: NonNullable<ReturnType<typeof useAgentsQuery>["data"]>;
  repositories: NonNullable<ReturnType<typeof useRepoRootsQuery>["data"]>;
  catalogError: boolean; selectAgent: (kind: string) => void; selectRepository: (id: string) => void;
};
function RuleEditor({ kind, repositoryId, agents, repositories, catalogError, selectAgent, selectRepository }: RuleEditorProps) {
  const editor = useAgentRulesEditor(kind, repositoryId);
  const locked = editor.busy || editor.dirty;
  return <div className="space-y-4" data-telemetry-block>
    <p className="text-ui-sm text-muted-foreground">Markdown instructions for one agent. Changes apply the next time an agent process starts, including resume.</p>
    <label className="block space-y-2"><span className="text-ui-sm">Agent</span>
      <Select value={kind} disabled={locked || !agents.length} onChange={(event) => selectAgent(event.currentTarget.value)}>
        {!agents.length ? <option value="">Connect a runtime to load agents</option> : null}
        {agents.map((agent) => <option key={agent.kind} value={agent.kind}>{agent.kind}</option>)}
      </Select>
    </label>
    <label className="block space-y-2"><span className="text-ui-sm">Scope</span>
      <Select value={repositoryId ?? ""} disabled={locked || !editor.access.available} onChange={(event) => selectRepository(event.currentTarget.value)}>
        <option value="">User default across repositories</option>
        {repositories.map((repository) => <option key={repository.id} value={repository.id}>{repository.path}</option>)}
      </Select>
    </label>
    {catalogError ? <p role="alert" className="text-ui-sm">Could not load agents or repositories. Reconnect the runtime and reopen Rules.</p> : null}
    {editor.access.query.isPending && kind && editor.access.available ? <p role="status" className="text-ui-sm">Loading rules…</p> : null}
    {editor.access.query.isError ? <div role="alert"><p className="text-ui-sm">Could not load rules.</p><Button variant="ghost" onClick={() => { void editor.access.query.refetch(); }}>Retry</Button></div> : null}
    {repositoryId ? <p className="text-ui-sm text-muted-foreground">{editor.access.query.data?.inherited ? "Using the user default." : "Repository rules replace the user default."} An empty saved file disables instructions for this repository.</p> : null}
    <label className="block space-y-2"><span className="text-ui-sm">Markdown instructions</span>
      <Textarea rows={14} value={editor.text} disabled={!editor.ready || editor.busy} spellCheck={false} onChange={(event) => editor.edit(event.currentTarget.value)} />
    </label>
    <p className="text-ui-xs text-muted-foreground">Up to 65,536 UTF-8 bytes. Stored as private Markdown files in this runtime. Instructions grant no tools or permissions.</p>
    <div className="flex flex-wrap gap-2">
      <Button size="sm" disabled={!editor.ready || editor.busy || !editor.dirty || !!editor.validation} onClick={() => { void editor.save(editor.text); }}>Save rules</Button>
      <Button size="sm" variant="secondary" disabled={!editor.ready || editor.busy} onClick={() => editor.input.current?.click()}>Import file</Button>
      <input ref={editor.input} type="file" accept=".md,.txt" hidden aria-label="Agent rules file" onChange={(event) => { void editor.importFile(event.currentTarget.files?.[0]); }} />
      {editor.dirty ? <Button size="sm" variant="ghost" disabled={editor.busy} onClick={editor.discard}>Discard edits</Button> : null}
      {repositoryId && !editor.access.query.data?.inherited ? <Button size="sm" variant="ghost" disabled={!editor.ready || locked} onClick={() => { void editor.save(null); }}>Use user default</Button> : null}
      {!repositoryId && editor.text ? <Button size="sm" variant="ghost" disabled={!editor.ready || locked} onClick={() => { void editor.save(null); }}>Clear default</Button> : null}
    </div>
    {editor.dirty ? <p role="status" className="text-ui-sm text-muted-foreground">Save or discard edits before changing agent or scope.</p> : null}
    {editor.validation || editor.error ? <p role="alert" className="text-ui-sm text-destructive">{editor.validation ?? editor.error}</p> : null}
  </div>;
}
