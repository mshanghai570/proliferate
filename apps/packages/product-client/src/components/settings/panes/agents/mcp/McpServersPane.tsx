import { useState, type FormEvent } from "react";
import { resolveRuntimeCacheScopeKey, useAnyHarnessRuntimeContext, useRepoRootsQuery } from "@anyharness/sdk-react";
import type { CreateMcpServerRequest, McpServerCredential, McpServerSummary } from "@anyharness/sdk";
import { Button } from "#product/primitives/Button";
import { Input } from "#product/primitives/Input";
import { Select } from "#product/primitives/Select";
import { Switch } from "#product/primitives/Switch";
import { Textarea } from "#product/primitives/Textarea";
import { NoticeBanner } from "#product/primitives/patterns/NoticeBanner";
import { useMcpServersAccess } from "#product/hooks/access/agents/use-mcp-servers-access";
import { useAgentSurfaceStore } from "#product/stores/ui/agent-surface-store";
import { MCP_SERVERS_COPY as copy } from "#product/copy/settings/agents/mcp-servers-copy";

type CredentialDraft = { name: string; value: string };

function emptyCredential(): CredentialDraft {
  return { name: "", value: "" };
}

function enteredCredentials(rows: CredentialDraft[]): McpServerCredential[] {
  return rows
    .filter((row) => row.name.trim() !== "" && row.value !== "")
    .map((row) => ({ name: row.name.trim(), value: row.value }));
}

function isDataKeyError(error: unknown): boolean {
  return typeof error === "object"
    && error !== null
    && "problem" in error
    && typeof error.problem === "object"
    && error.problem !== null
    && "code" in error.problem
    && error.problem.code === "MCP_SERVER_DATA_KEY_REQUIRED";
}

export function McpServersPane() {
  const runtime = useAnyHarnessRuntimeContext();
  const local = useAgentSurfaceStore((state) => state.surface === "local");
  if (!local) {
    return (
      <>
        <h1 className="text-ui font-medium">{copy.title}</h1>
        <NoticeBanner>{copy.localOnly}</NoticeBanner>
      </>
    );
  }
  return <McpServerControls key={`${resolveRuntimeCacheScopeKey(runtime)}:${runtime.runtimeUrl}`} />;
}

function McpServerControls() {
  const access = useMcpServersAccess();
  const repositories = useRepoRootsQuery({ enabled: access.available });
  const [repositoryId, setRepositoryId] = useState("");
  const [serverName, setServerName] = useState("");
  const [description, setDescription] = useState("");
  const [transport, setTransport] = useState<"stdio" | "http">("stdio");
  const [command, setCommand] = useState("");
  const [args, setArgs] = useState("");
  const [url, setUrl] = useState("");
  const [env, setEnv] = useState<CredentialDraft[]>([emptyCredential()]);
  const [headers, setHeaders] = useState<CredentialDraft[]>([emptyCredential()]);
  const [removeId, setRemoveId] = useState<string | null>(null);
  const [formError, setFormError] = useState<string | null>(null);
  const busy = access.mutation.isPending;
  const servers = access.query.data?.servers ?? [];

  function createServer(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setFormError(null);
    const config: CreateMcpServerRequest["config"] = transport === "stdio"
      ? {
        transport: "stdio",
        command: command.trim(),
        args: args === "" ? [] : args.split(/\r?\n/),
        env: enteredCredentials(env),
      }
      : {
        transport: "http",
        url: url.trim(),
        headers: enteredCredentials(headers),
      };
    const request: CreateMcpServerRequest = {
      serverName: serverName.trim(),
      ...(description.trim() ? { description: description.trim() } : {}),
      config,
    };
    void access.mutation.mutateAsync({ kind: "create", request }).then(() => {
      setServerName("");
      setDescription("");
      setCommand("");
      setArgs("");
      setUrl("");
      setEnv([emptyCredential()]);
      setHeaders([emptyCredential()]);
    }).catch((error: unknown) => {
      setFormError(isDataKeyError(error) ? copy.unavailableKey : copy.createError);
    });
  }

  function setEnabled(server: McpServerSummary, enabled: boolean | null) {
    const request = repositoryId
      ? { repoRootId: repositoryId, enabled }
      : enabled ?? false;
    access.mutation.mutate({ kind: "enable", id: server.id, request });
  }

  function renderCredentialRows(
    label: string,
    rows: CredentialDraft[],
    setRows: (rows: CredentialDraft[]) => void,
  ) {
    return (
      <fieldset className="space-y-2">
        <legend className="text-ui-sm">{label}</legend>
        {rows.map((row, index) => (
          <div className="flex flex-wrap items-end gap-2" key={index}>
            <label className="min-w-32 flex-1 space-y-1">
              <span className="text-ui-xs text-muted-foreground">{copy.credentialName}</span>
              <Input value={row.name} autoComplete="off" onChange={(event) => {
                const next = [...rows];
                next[index] = { ...row, name: event.currentTarget.value };
                setRows(next);
              }} />
            </label>
            <label className="min-w-32 flex-1 space-y-1">
              <span className="text-ui-xs text-muted-foreground">{copy.credentialValue}</span>
              <Input type="password" autoComplete="new-password" value={row.value} onChange={(event) => {
                const next = [...rows];
                next[index] = { ...row, value: event.currentTarget.value };
                setRows(next);
              }} />
            </label>
            {rows.length > 1 ? (
              <Button type="button" variant="ghost" size="sm" onClick={() => setRows(rows.filter((_, rowIndex) => rowIndex !== index))}>
                {copy.removeCredential}
              </Button>
            ) : null}
          </div>
        ))}
        <Button type="button" variant="ghost" size="sm" disabled={busy} onClick={() => setRows([...rows, emptyCredential()])}>
          {copy.addCredential}
        </Button>
      </fieldset>
    );
  }

  return (
    <div className="space-y-5">
      <header className="space-y-2">
        <h1 className="text-ui font-medium">{copy.title}</h1>
        <p className="text-ui-sm text-muted-foreground">{copy.description}</p>
      </header>

      <form onSubmit={createServer}>
        <fieldset disabled={!access.available || busy} className="space-y-4 rounded-md border border-border p-4">
          <label className="block space-y-1">
            <span className="text-ui-sm">{copy.serverName}</span>
            <Input required maxLength={100} value={serverName} onChange={(event) => setServerName(event.currentTarget.value)} />
          </label>
          <label className="block space-y-1">
            <span className="text-ui-sm">{copy.descriptionLabel}</span>
            <Input value={description} onChange={(event) => setDescription(event.currentTarget.value)} />
          </label>
          <label className="block space-y-1">
            <span className="text-ui-sm">{copy.transport}</span>
            <Select value={transport} disabled={busy} onChange={(event) => setTransport(event.currentTarget.value as "stdio" | "http")}>
              <option value="stdio">{copy.stdio}</option>
              <option value="http">{copy.http}</option>
            </Select>
          </label>
          {transport === "stdio" ? (
            <>
              <label className="block space-y-1">
                <span className="text-ui-sm">{copy.command}</span>
                <Input required value={command} onChange={(event) => setCommand(event.currentTarget.value)} />
              </label>
              <label className="block space-y-1">
                <span className="text-ui-sm">{copy.args}</span>
                <Textarea rows={3} value={args} onChange={(event) => setArgs(event.currentTarget.value)} />
              </label>
              {renderCredentialRows(copy.env, env, setEnv)}
            </>
          ) : (
            <>
              <label className="block space-y-1">
                <span className="text-ui-sm">{copy.url}</span>
                <Input required type="url" value={url} onChange={(event) => setUrl(event.currentTarget.value)} />
              </label>
              {renderCredentialRows(copy.headers, headers, setHeaders)}
            </>
          )}
          <p className="text-ui-xs text-muted-foreground">{copy.secretsNote}</p>
          <Button type="submit" variant="secondary" disabled={!access.available || busy}>
            {busy ? copy.saving : copy.addServer}
          </Button>
        </fieldset>
        {formError ? <p role="alert" className="text-ui-sm text-destructive">{formError}</p> : null}
      </form>

      <label className="block space-y-2">
        <span className="text-ui-sm">{copy.scope}</span>
        <Select value={repositoryId} disabled={busy || !access.available} onChange={(event) => {
          setRepositoryId(event.currentTarget.value);
          setRemoveId(null);
        }}>
          <option value="">{copy.userDefault}</option>
          {(repositories.data ?? []).map((repository) => (
            <option key={repository.id} value={repository.id}>{repository.path}</option>
          ))}
        </Select>
      </label>
      {repositoryId ? <p className="text-ui-sm text-muted-foreground">{copy.repoOverrideHelp}</p> : null}
      {!access.available ? <p className="text-ui-sm text-muted-foreground">{copy.disconnected}</p> : null}
      {access.query.isPending && access.available ? <p role="status" className="text-ui-sm text-muted-foreground">Loading MCP servers…</p> : null}
      {access.query.isError ? (
        <div role="alert">
          <p className="text-ui-sm">{copy.loadError}</p>
          <Button type="button" variant="ghost" size="sm" onClick={() => { void access.query.refetch(); }}>{copy.retry}</Button>
        </div>
      ) : null}
      {repositories.isError ? <p role="alert" className="text-ui-sm">Could not load repositories. Retry after reconnecting to the runtime.</p> : null}
      {access.mutation.isError && !formError ? <p role="alert" className="text-ui-sm text-destructive">{copy.mutationError}</p> : null}
      {access.query.isSuccess && servers.length === 0 ? <p className="text-ui-sm text-muted-foreground">{copy.empty}</p> : null}

      <ul className="divide-y divide-border">
        {servers.map((server) => {
          const override = repositoryId ? server.repositoryOverrides[repositoryId] : undefined;
          const enabled = repositoryId
            ? override ?? server.userEnabled ?? false
            : server.userEnabled ?? false;
          return (
            <li key={server.id} className="space-y-2 py-3">
              <div className="flex flex-wrap items-center gap-3">
                <div className="min-w-0 flex-1">
                  <p className="truncate text-ui">{server.serverName}</p>
                  <p className="text-ui-xs text-muted-foreground">
                    {server.transport === "stdio" ? copy.stdio : copy.http}
                    {repositoryId && override === undefined ? ` · ${copy.usingDefault}` : ""}
                  </p>
                </div>
                <Switch
                  checked={enabled}
                  disabled={busy || !access.available}
                  aria-label={`${copy.enabled} ${server.serverName}`}
                  onChange={(value) => setEnabled(server, value)}
                />
                {repositoryId && override !== undefined ? (
                  <Button type="button" variant="ghost" size="sm" disabled={busy} onClick={() => setEnabled(server, null)}>
                    {copy.useDefault}
                  </Button>
                ) : null}
                <Button type="button" variant="ghost" size="sm" disabled={busy} onClick={() => setRemoveId(server.id)}>
                  {copy.remove}
                </Button>
              </div>
              {removeId === server.id ? (
                <div className="flex flex-wrap items-center gap-2">
                  <p className="text-ui-sm">{copy.deleteQuestion}</p>
                  <Button type="button" variant="secondary" size="sm" disabled={busy} onClick={() => {
                    access.mutation.mutate({ kind: "remove", id: server.id }, {
                      onSuccess: () => setRemoveId(null),
                    });
                  }}>{copy.confirmRemove}</Button>
                  <Button type="button" variant="ghost" size="sm" disabled={busy} onClick={() => setRemoveId(null)}>{copy.cancel}</Button>
                </div>
              ) : null}
            </li>
          );
        })}
      </ul>
    </div>
  );
}
