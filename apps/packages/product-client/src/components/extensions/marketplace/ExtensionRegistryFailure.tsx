import { Button } from "#product/primitives/Button";
import { EXTENSION_MARKETPLACE_COPY as copy } from "#product/copy/extensions/marketplace";
import { ExtensionRegistryError } from "#product/lib/domain/extensions/extension-registry-error";

export function ExtensionRegistryFailure({ error, onRetry }: { error: unknown; onRetry: () => void }) {
  const code = error instanceof ExtensionRegistryError ? error.code : "network-error";
  return (
    <div role="alert" className="space-y-3 p-5 text-ui text-muted-foreground">
      <p>{copy.errors[code]}</p>
      <Button variant="secondary" onClick={onRetry}>{copy.retry}</Button>
    </div>
  );
}
