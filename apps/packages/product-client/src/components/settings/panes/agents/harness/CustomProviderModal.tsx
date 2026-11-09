import { useState, type FormEvent } from "react";
import type { CreateAgentProviderConfigRequest } from "@proliferate/cloud-sdk";
import { Button } from "#product/primitives/Button";
import { Input } from "#product/primitives/Input";
import { Label } from "#product/primitives/Label";
import { Select } from "#product/primitives/Select";
import { Textarea } from "#product/primitives/Textarea";
import { NoticeBanner } from "#product/primitives/patterns/NoticeBanner";
import { ModalShell } from "#product/primitives/patterns/ModalShell";

interface CustomProviderModalProps {
  onClose: () => void;
  onSubmit: (input: CreateAgentProviderConfigRequest) => void;
  busy: boolean;
  error: string | null;
}

export function CustomProviderModal({ onClose, onSubmit, busy, error }: CustomProviderModalProps) {
  const [name, setName] = useState("");
  const [endpoint, setEndpoint] = useState("");
  const [apiKey, setApiKey] = useState("");
  const [models, setModels] = useState("");
  const [apiFormat, setApiFormat] = useState("chat");
  const canSubmit = !busy && Boolean(name.trim() && endpoint.trim() && models.trim());

  function submit(event: FormEvent) {
    event.preventDefault();
    if (!canSubmit) return;
    onSubmit({ title: name.trim(), kind: "openai_compatible", value: {
      name: name.trim(), endpoint: endpoint.trim(), models: models.trim(), apiFormat,
      ...(apiKey.trim() ? { apiKey: apiKey.trim() } : {}),
    } });
  }

  return (
    <ModalShell open onClose={onClose} disableClose={busy} telemetryBlocked
      title="Add OpenAI-compatible provider"
      description="Use your own endpoint with OpenCode. Models appear after the runtime refreshes its available models."
    >
      <form className="space-y-4" onSubmit={submit}>
        <div className="space-y-1.5">
          <Label htmlFor="custom-provider-name">Name</Label>
          <Input id="custom-provider-name" value={name} maxLength={255} required disabled={busy}
            autoComplete="off" placeholder="Local models" onChange={(event) => setName(event.target.value)} />
        </div>
        <div className="space-y-1.5">
          <Label htmlFor="custom-provider-endpoint">API base URL</Label>
          <Input id="custom-provider-endpoint" type="url" value={endpoint} required disabled={busy}
            autoComplete="off" spellCheck={false} placeholder="http://localhost:1234/v1"
            aria-describedby="custom-provider-endpoint-help" onChange={(event) => setEndpoint(event.target.value)} />
          <p id="custom-provider-endpoint-help" className="text-ui-sm text-muted-foreground">
            Include the full API base path, such as /v1.
          </p>
        </div>
        <div className="space-y-1.5">
          <Label htmlFor="custom-provider-format">API format</Label>
          <Select id="custom-provider-format" value={apiFormat} disabled={busy} onChange={(event) => setApiFormat(event.target.value)}>
            <option value="chat">Chat Completions</option>
            <option value="responses">Responses</option>
          </Select>
        </div>
        <div className="space-y-1.5">
          <Label htmlFor="custom-provider-key">API key (optional)</Label>
          <Input id="custom-provider-key" type="password" value={apiKey} disabled={busy} data-telemetry-mask
            autoComplete="off" spellCheck={false} onChange={(event) => setApiKey(event.target.value)} />
        </div>
        <div className="space-y-1.5">
          <Label htmlFor="custom-provider-models">Model IDs</Label>
          <Textarea id="custom-provider-models" value={models} required disabled={busy} rows={3}
            spellCheck={false} placeholder="org/model-name" aria-describedby="custom-provider-models-help"
            onChange={(event) => setModels(event.target.value)} />
          <p id="custom-provider-models-help" className="text-ui-sm text-muted-foreground">One model ID per line.</p>
        </div>
        {error ? <NoticeBanner tone="destructive">{error}</NoticeBanner> : null}
        <div className="flex justify-end gap-2">
          <Button type="button" variant="ghost" disabled={busy} onClick={onClose}>Cancel</Button>
          <Button type="submit" disabled={!canSubmit} loading={busy}>Add provider</Button>
        </div>
      </form>
    </ModalShell>
  );
}
