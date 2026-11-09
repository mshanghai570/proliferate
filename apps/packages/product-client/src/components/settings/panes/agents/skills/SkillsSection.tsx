import { useRef, useState, type InputHTMLAttributes } from "react";
import { resolveRuntimeCacheScopeKey, useAnyHarnessRuntimeContext, useRepoRootsQuery } from "@anyharness/sdk-react";
import { Button } from "#product/primitives/Button";
import { Select } from "#product/primitives/Select";
import { Switch } from "#product/primitives/Switch";
import { useSkillsAccess } from "#product/hooks/access/skills/use-skills-access";
import { importSkillFiles } from "#product/hooks/skills/workflows/import-skill-files";
import { useAgentSurfaceStore } from "#product/stores/ui/agent-surface-store";
import { NoticeBanner } from "#product/primitives/patterns/NoticeBanner";

const directoryInput = { webkitdirectory: "", directory: "" } as InputHTMLAttributes<HTMLInputElement>;

export function SkillsSection() {
  const runtime = useAnyHarnessRuntimeContext();
  const local = useAgentSurfaceStore((state) => state.surface === "local");
  if (!local) return <><h1 className="text-title font-semibold">Agent skills</h1><NoticeBanner>Skills are configured for local sessions. Select Local to manage them.</NoticeBanner></>;
  return <SkillControls key={`${resolveRuntimeCacheScopeKey(runtime)}:${runtime.runtimeUrl}`} />;
}

function SkillControls() {
  const access = useSkillsAccess();
  const repositories = useRepoRootsQuery({ enabled: access.available });
  const input = useRef<HTMLInputElement>(null);
  const [repositoryId, setRepositoryId] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [reading, setReading] = useState(false);
  const [removeId, setRemoveId] = useState<string | null>(null);
  const busy = reading || access.mutation.isPending;
  const skills = access.query.data?.skills ?? [];

  const importFiles = async (files: File[]) => {
    setError(null);
    setReading(true);
    try {
      const request = await importSkillFiles(files);
      await access.mutation.mutateAsync({ kind: "import", request });
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : "Could not import the skill. Choose the folder again.");
    } finally {
      setReading(false);
      if (input.current) input.current.value = "";
    }
  };
  const change = (id: string, enabled: boolean | null) => {
    setError(null);
    access.mutation.mutate({ kind: "enable", id, request: { repositoryId: repositoryId || null, enabled } });
  };

  return (
    <div className="space-y-5">
      <div className="space-y-2">
        <h1 className="text-ui font-medium">Agent skills</h1>
        <p className="text-ui-sm text-muted-foreground">Import a folder containing SKILL.md and its resources. Enabled skills apply when an agent starts. They grant no tools or permissions.</p>
        <Button type="button" variant="secondary" size="sm" disabled={!access.available || busy} onClick={() => input.current?.click()}>
          {reading ? "Reading folder…" : "Import skill folder"}
        </Button>
        <input {...directoryInput} ref={input} type="file" multiple hidden aria-label="Skill package folder"
          onChange={(event) => { void importFiles(Array.from(event.currentTarget.files ?? [])); }} />
        <p className="text-ui-xs text-muted-foreground">Up to 128 files and 1 MiB per package. Imports start disabled.</p>
      </div>
      <label className="block space-y-2">
        <span className="text-ui-sm">Enablement scope</span>
        <Select value={repositoryId} disabled={busy || !access.available} onChange={(event) => { setRepositoryId(event.currentTarget.value); setRemoveId(null); }}>
          <option value="">User default across repositories</option>
          {(repositories.data ?? []).map((repository) => <option key={repository.id} value={repository.id}>{repository.path}</option>)}
        </Select>
      </label>
      {repositoryId ? <p className="text-ui-sm text-muted-foreground">Repository choices override your user default, including for worktrees. Use the default to remove an override.</p> : null}
      {!access.available ? <p className="text-ui-sm text-muted-foreground">Connect to a runtime to manage its skills.</p> : null}
      {access.query.isPending && access.available ? <p role="status" className="text-ui-sm text-muted-foreground">Loading skills…</p> : null}
      {access.query.isError ? <div role="alert"><p className="text-ui-sm">Could not load skills.</p><Button variant="ghost" size="sm" onClick={() => { void access.query.refetch(); }}>Retry</Button></div> : null}
      {repositories.isError ? <p role="alert" className="text-ui-sm">Could not load repositories. Retry after reconnecting to the runtime.</p> : null}
      {error || access.mutation.isError ? <p role="alert" className="text-ui-sm text-destructive">{error ?? "Could not save skills. Check the runtime connection and try again."}</p> : null}
      {access.query.isSuccess && skills.length === 0 ? <p className="text-ui-sm text-muted-foreground">No skills imported.</p> : null}
      <ul className="divide-y divide-border">
        {skills.map((skill) => {
          const override = repositoryId ? skill.repositoryOverrides[repositoryId] : undefined;
          const enabled = override ?? skill.userEnabled;
          return <li key={skill.id} className="space-y-2 py-3">
            <div className="flex flex-wrap items-center gap-3">
              <div className="min-w-0 flex-1"><p className="truncate text-ui">{skill.name}</p><p className="text-ui-xs text-muted-foreground">{skill.fileCount} files{repositoryId && override === undefined ? " · Using user default" : ""}</p></div>
              <Switch checked={enabled} disabled={busy} aria-label={`Enable ${skill.name}`} onChange={(value) => change(skill.id, value)} />
              {repositoryId && override !== undefined ? <Button variant="ghost" size="sm" disabled={busy} onClick={() => change(skill.id, null)}>Use default</Button> : null}
              <Button variant="ghost" size="sm" disabled={busy} onClick={() => setRemoveId(skill.id)}>Remove</Button>
            </div>
            {removeId === skill.id ? <div className="flex flex-wrap items-center gap-2"><p className="text-ui-sm">Remove this skill from every scope?</p><Button variant="secondary" size="sm" disabled={busy} onClick={() => access.mutation.mutate({ kind: "remove", id: skill.id }, { onSuccess: () => setRemoveId(null) })}>Remove skill</Button><Button variant="ghost" size="sm" disabled={busy} onClick={() => setRemoveId(null)}>Cancel</Button></div> : null}
          </li>;
        })}
      </ul>
    </div>
  );
}
