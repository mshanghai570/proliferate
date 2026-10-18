import { Button } from "#product/primitives/Button";
import { Input } from "#product/primitives/Input";
import { Textarea } from "#product/primitives/Textarea";
import { SettingsSection } from "#product/primitives/patterns/settings/SettingsSection";
import { SettingsRow } from "#product/primitives/patterns/settings/SettingsRow";
import { ConfirmationDialog } from "#product/primitives/patterns/ConfirmationDialog";
import { useProjectMcpEditor, useProjectMcpEditorIdentity } from "#product/hooks/settings/workflows/use-project-mcp-editor";

const HTTP_EXAMPLE = JSON.stringify({ transport: "http", url: "http://localhost:8000/mcp", headers: {} }, null, 2);
const STDIO_EXAMPLE = JSON.stringify({ transport: "stdio", command: "/absolute/path/to/mcp-server", args: [], env: {} }, null, 2);

export function ProjectMcpSettings({ repoRootId }: { repoRootId: string }) {
  const identity = useProjectMcpEditorIdentity(repoRootId);
  return <ProjectMcpEditor key={identity} repoRootId={repoRootId} />;
}

function ProjectMcpEditor({ repoRootId }: { repoRootId: string }) {
  const editor = useProjectMcpEditor(repoRootId);
  const { query } = editor;
  const blocked = editor.pending || !editor.connected || query.isPending || query.isError;
  const servers = query.data?.servers ?? [];
  return <SettingsSection
    title="Project MCP servers"
    description="Share HTTP and stdio tools with this repository's local sessions and worktrees. Changes apply at the next session launch. Running sessions keep their current tools."
  >
    {!editor.connected ? <p className="px-3.5 py-3 text-ui-sm text-muted-foreground">Connect the local runtime to manage MCP servers.</p>
      : query.isPending ? <p role="status" className="px-3.5 py-3 text-ui-sm text-muted-foreground">Loading MCP servers…</p>
      : query.isError ? <div className="flex items-center gap-2 px-3.5 py-3">
        <p role="alert" className="text-ui-sm text-destructive">Could not load project MCP settings.</p>
        <Button variant="secondary" onClick={() => { void query.refetch(); }}>Retry</Button>
      </div> : <>
        {servers.length === 0 && <p className="px-3.5 py-3 text-ui-sm text-muted-foreground">No project MCP servers configured.</p>}
        <ul aria-label="Project MCP servers">
          {servers.map((server) => <li key={server.name}>
            <SettingsRow label={server.name} description={`${server.transport === "http" ? "HTTP" : "Stdio"} · ${server.enabled ? "Enabled for new launches" : "Disabled for new launches"}`}>
              <Button type="button" variant="secondary" disabled={blocked} aria-label={`${server.enabled ? "Disable" : "Enable"} ${server.name}`} onClick={() => editor.toggle(server.name, !server.enabled)}>{server.enabled ? "Disable" : "Enable"}</Button>
              <Button type="button" variant="ghost" disabled={blocked} aria-label={`Remove ${server.name}`} onClick={() => editor.setConfirm({ action: "remove", name: server.name })}>Remove</Button>
            </SettingsRow>
          </li>)}
        </ul>
      </>}
    <form className="flex flex-col gap-3 px-3.5 py-3" onSubmit={(event) => { event.preventDefault(); editor.save(); }}>
      <label className="flex flex-col gap-1 text-ui" htmlFor="project-mcp-name">
        Server name
        <Input id="project-mcp-name" value={editor.name} maxLength={50} autoComplete="off" disabled={blocked} onChange={(event) => editor.setName(event.target.value)} placeholder="project-tools" />
      </label>
      <div className="flex flex-wrap gap-2">
        <Button type="button" variant="ghost" disabled={blocked || !!editor.source} onClick={() => editor.setSource(HTTP_EXAMPLE)}>HTTP example</Button>
        <Button type="button" variant="ghost" disabled={blocked || !!editor.source} onClick={() => editor.setSource(STDIO_EXAMPLE)}>Stdio example</Button>
      </div>
      <label className="flex flex-col gap-1 text-ui" htmlFor="project-mcp-config">
        Server configuration (JSON)
        <Textarea id="project-mcp-config" value={editor.source} rows={8} spellCheck={false} autoComplete="off" disabled={blocked} aria-describedby="project-mcp-help" onChange={(event) => editor.setSource(event.target.value)} />
      </label>
      <p id="project-mcp-help" className="text-ui-sm text-muted-foreground">Use transport "http" with url and optional headers, or "stdio" with command, args, and env. Headers and env are objects with string values. Stdio commands run on your Mac with your account's permissions. Configure servers you trust. Credentials are encrypted and cannot be read back here; replacing a server requires its full configuration and enables it.</p>
      {query.data && !query.data.encryptionAvailable && <p role="alert" className="text-ui-sm text-destructive">The runtime encryption key is unavailable. Restart the desktop app before adding a server.</p>}
      {editor.error && <p role="alert" className="text-ui-sm text-destructive">{editor.error}</p>}
      {editor.notice && <p role="status" className="text-ui-sm text-muted-foreground">{editor.notice}</p>}
      <div><Button type="submit" loading={editor.pending} disabled={blocked || !query.data?.encryptionAvailable || !editor.name || !editor.source}>Save server</Button></div>
    </form>
    <ConfirmationDialog
      open={editor.confirm !== null}
      title={editor.confirm?.action === "remove" ? `Remove ${editor.confirm.name}?` : `Replace ${editor.confirm?.name ?? "server"}?`}
      description={editor.confirm?.action === "remove" ? "Remove this server and its saved credentials from the project. Running sessions keep their current tools until their process stops." : "Replace the entire saved configuration and credentials, and enable this server for future launches."}
      confirmLabel={editor.confirm?.action === "remove" ? "Remove server" : "Replace server"}
      confirmVariant={editor.confirm?.action === "remove" ? "destructive" : "primary"}
      loading={editor.pending}
      disableClose={editor.pending}
      onClose={() => editor.setConfirm(null)}
      onConfirm={editor.confirmAction}
    />
  </SettingsSection>;
}
