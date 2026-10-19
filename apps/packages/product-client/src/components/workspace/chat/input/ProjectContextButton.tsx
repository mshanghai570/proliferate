import { useState } from "react";
import { resolveRuntimeCacheScopeKey, useAnyHarnessRuntimeContext } from "@anyharness/sdk-react";
import { Button } from "#product/primitives/Button";
import { Input } from "#product/primitives/Input";
import { ModalShell } from "#product/primitives/patterns/ModalShell";
import { useAgentSurfaceStore } from "#product/stores/ui/agent-surface-store";
import { useProjectContextPicker } from "#product/hooks/chat/workflows/use-project-context-picker";

type Props = { workspaceId: string; disabled: boolean; onAdd: (text: string) => void };
export function ProjectContextButton(props: Props) {
  const runtime = useAnyHarnessRuntimeContext();
  const local = useAgentSurfaceStore((state) => state.surface === "local");
  if (!local) return null;
  return <ContextLauncher key={`${resolveRuntimeCacheScopeKey(runtime)}:${runtime.runtimeUrl}:${props.workspaceId}`} {...props} />;
}
function ContextLauncher({ workspaceId, disabled, onAdd }: Props) {
  const [open, setOpen] = useState(false);
  return <>
    <Button type="button" size="sm" variant="ghost" className="self-start" disabled={disabled} onClick={() => setOpen(true)}>Project context</Button>
    {open && !disabled ? <ContextDialog workspace={workspaceId} onAdd={onAdd} onClose={() => setOpen(false)} /> : null}
  </>;
}
function ContextDialog({ workspace, onAdd, onClose }: { workspace: string; onAdd: (text: string) => void; onClose: () => void }) {
  const picker = useProjectContextPicker(workspace, onAdd);
  const { access } = picker;
  const status = access.query.data;
  const ready = access.available && access.query.isSuccess && !access.query.isError;
  const failure = access.refresh.isError || access.clear.isError || access.search.isError;
  return <ModalShell open onClose={onClose} title="Project context" sizeClassName="max-w-2xl max-h-[84vh]" bodyClassName="overflow-y-auto px-5 pb-5 pt-4" telemetryBlocked
    description="Search local project source, review excerpts, and add selected text to your draft. Sending the prompt shares that text with the selected provider."
    footer={<><Button type="button" variant="ghost" onClick={onClose}>Cancel</Button>
      <Button type="button" disabled={!ready || picker.busy || !picker.selected.length} onClick={() => { if (picker.add()) onClose(); }}>Add context to draft</Button>
    </>}>
    <div className="space-y-4">
      <p className="text-ui-sm text-muted-foreground">Local keyword retrieval (RAG). Indexes supported text files using Git ignores and credential exclusions. Source files can still contain private data. No model is called during indexing or search.</p>
      {!access.available ? <p role="status" className="text-ui-sm">Connect a local runtime to use project context.</p> : null}
      {access.query.isPending && access.available ? <p role="status">Loading index…</p> : null}
      {access.query.isError ? <div role="alert"><p>Could not read the index. Retry the runtime connection, or clear the local index and index again.</p><Button type="button" variant="ghost" onClick={() => { void access.query.refetch(); }}>Retry</Button></div> : null}
      {status?.enabled ? <p className="text-ui-sm">{status.indexedFiles} files · {status.chunks} excerpts · {status.skippedFiles} skipped{status.indexedAt ? ` · Updated ${new Date(status.indexedAt).toLocaleString()}` : ""}</p> : <p className="text-ui-sm">Index this Git workspace to search its source.</p>}
      {status?.truncated ? <p role="status" className="text-ui-sm">This index is partial because it reached a size or time limit.</p> : null}
      <div className="flex flex-wrap gap-2">
        <Button type="button" size="sm" variant="secondary" disabled={!ready || picker.busy} onClick={picker.index}>{status?.enabled ? "Refresh index" : "Index project"}</Button>
        {status?.enabled || access.query.isError ? <Button type="button" size="sm" variant="ghost" disabled={!access.available || picker.busy} onClick={picker.clear}>Clear local index</Button> : null}
      </div>
      <label className="block space-y-2"><span className="text-ui-sm">Search source</span>
        <Input value={picker.query} disabled={!ready || !status?.enabled || picker.busy} placeholder="Authentication session refresh" onChange={(event) => picker.editQuery(event.currentTarget.value)}
          onKeyDown={(event) => { if (event.key === "Enter" && !event.nativeEvent.isComposing) { event.preventDefault(); event.stopPropagation(); if (ready && status?.enabled && !picker.busy && picker.query.trim()) picker.search(); } }} />
      </label>
      <Button type="button" size="sm" disabled={!ready || !status?.enabled || picker.busy || !picker.query.trim()} onClick={picker.search}>Search</Button>
      {picker.busy ? <p role="status" className="text-ui-sm">{access.search.isPending ? "Refreshing and searching source…" : "Updating local index…"}</p> : null}
      {failure || picker.error ? <p role="alert" className="text-ui-sm text-destructive">{picker.error ?? "Could not update or search the project. Check access to the local Git workspace and retry with 1–32 terms (up to 512 UTF-8 bytes)."}</p> : null}
      {access.search.isSuccess && !picker.excerpts.length ? <p role="status" className="text-ui-sm">No matching excerpts. Try other words or check the index limits.</p> : null}
      <ul className="space-y-3">
        {picker.excerpts.map((excerpt, index) => <li key={`${excerpt.path}:${excerpt.startLine}`} className="space-y-2">
          <label className="flex items-start gap-2 text-ui-sm"><Input type="checkbox" variant="unstyled" checked={picker.selected.includes(index)} disabled={picker.busy} onChange={() => picker.toggle(index)} />
            <span>{excerpt.path}:{excerpt.startLine}-{excerpt.endLine}</span>
          </label>
          <pre className="overflow-x-auto whitespace-pre-wrap rounded-md border border-border p-3 font-mono text-ui-xs">{excerpt.text}</pre>
        </li>)}
      </ul>
    </div>
  </ModalShell>;
}
