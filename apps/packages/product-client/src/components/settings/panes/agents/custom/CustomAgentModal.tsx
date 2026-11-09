import { useState, type FormEvent } from "react";
import type { RegisterCustomAcpAgentRequest } from "@anyharness/sdk";
import { Button } from "#product/primitives/Button";
import { Input } from "#product/primitives/Input";
import { Label } from "#product/primitives/Label";
import { Textarea } from "#product/primitives/Textarea";
import { NoticeBanner } from "#product/primitives/patterns/NoticeBanner";
import { ModalShell } from "#product/primitives/patterns/ModalShell";
import { parseCustomAgentForm } from "#product/lib/domain/agents/custom-agent-form";

interface CustomAgentModalProps {
  onClose: () => void;
  onSubmit: (request: RegisterCustomAcpAgentRequest) => void;
  busy: boolean;
  error: string | null;
}

export function CustomAgentModal({ onClose, onSubmit, busy, error }: CustomAgentModalProps) {
  const [name, setName] = useState("");
  const [command, setCommand] = useState("");
  const [args, setArgs] = useState("[]");
  const [env, setEnv] = useState("{}");
  const [validationError, setValidationError] = useState<string | null>(null);
  function submit(event: FormEvent) {
    event.preventDefault();
    if (busy) return;
    setValidationError(null);
    try {
      onSubmit(parseCustomAgentForm(name, command, args, env));
    } catch (error) {
      setValidationError(error instanceof Error ? error.message : "Check the agent configuration.");
    }
  }
  return (
    <ModalShell open onClose={onClose} disableClose={busy} telemetryBlocked
      title="Connect ACP agent" description="Connect an installed agent that speaks Agent Client Protocol over standard input and output.">
      <form className="space-y-4" onSubmit={submit}>
        <div className="space-y-1.5">
          <Label htmlFor="custom-agent-name">Name</Label>
          <Input id="custom-agent-name" value={name} required maxLength={120} disabled={busy}
            autoComplete="off" onChange={(event) => setName(event.target.value)} />
        </div>
        <div className="space-y-1.5">
          <Label htmlFor="custom-agent-command">Executable</Label>
          <Input id="custom-agent-command" value={command} required disabled={busy} autoComplete="off"
            spellCheck={false} placeholder="/path/to/agent" aria-describedby="custom-agent-command-help"
            onChange={(event) => setCommand(event.target.value)} />
          <p id="custom-agent-command-help" className="text-ui-sm text-muted-foreground">
            Use an absolute path or a command on PATH. Arguments are passed separately, without shell expansion.
          </p>
        </div>
        <div className="space-y-1.5">
          <Label htmlFor="custom-agent-args">Arguments (JSON array)</Label>
          <Textarea id="custom-agent-args" value={args} rows={3} disabled={busy} spellCheck={false}
            placeholder={'["--acp"]'} onChange={(event) => setArgs(event.target.value)} />
        </div>
        <div className="space-y-1.5">
          <Label htmlFor="custom-agent-env">Environment (JSON object)</Label>
          <Textarea id="custom-agent-env" value={env} rows={3} disabled={busy} spellCheck={false} data-telemetry-mask
            aria-describedby="custom-agent-env-help" onChange={(event) => setEnv(event.target.value)} />
          <p id="custom-agent-env-help" className="text-ui-sm text-muted-foreground">
            Optional string values, saved on this machine. Values are hidden after saving. The agent runs with access to the workspace.
          </p>
        </div>
        {validationError || error ? <NoticeBanner tone="destructive">{validationError || error}</NoticeBanner> : null}
        <div className="flex justify-end gap-2">
          <Button type="button" variant="ghost" disabled={busy} onClick={onClose}>Cancel</Button>
          <Button type="submit" disabled={busy || !name.trim() || !command.trim()} loading={busy}>Connect agent</Button>
        </div>
      </form>
    </ModalShell>
  );
}
