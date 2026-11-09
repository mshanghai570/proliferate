import { Button } from "#product/primitives/Button";
import { Badge } from "#product/primitives/Badge";
import { Blocks } from "#product/primitives/icons/platform";
import { resolveExtensionContentUrl } from "#product/lib/domain/extensions/extension-content";
import type { ExtensionSummary } from "#product/lib/domain/extensions/extension-models";

export function ExtensionCard({ extension, onSelect }: { extension: ExtensionSummary; onSelect: () => void }) {
  const icon = resolveExtensionContentUrl(extension.iconUrl);
  return (
    <Button variant="unstyled" size="unstyled" onClick={onSelect}
      className="flex w-full items-start gap-3 whitespace-normal rounded-md p-4 text-left hover:bg-hover focus-visible:outline focus-visible:outline-2 focus-visible:outline-ring">
      <div className="relative flex size-12 shrink-0 items-center justify-center overflow-hidden rounded-md bg-surface-control text-muted-foreground">
        <Blocks className="icon-large" aria-hidden />
        {icon && <img src={icon} alt="" loading="lazy" referrerPolicy="no-referrer" className="absolute inset-0 size-full object-contain" onError={(event) => { event.currentTarget.hidden = true; }} />}
      </div>
      <div className="min-w-0 flex-1 space-y-1">
        <div className="flex flex-wrap items-center gap-2">
          <span className="text-ui font-semibold text-foreground">{extension.displayName}</span>
          {extension.deprecated && <Badge tone="warning">Deprecated</Badge>}
        </div>
        <p className="text-ui-sm text-muted-foreground">{extension.ref.namespace} · {extension.version}</p>
        <p className="line-clamp-2 text-ui text-muted-foreground">{extension.description}</p>
        <div className="flex flex-wrap gap-3 text-ui-sm text-muted-foreground">
          {extension.downloadCount !== null && <span>{extension.downloadCount.toLocaleString()} downloads</span>}
          {extension.averageRating !== null && <span>{extension.averageRating.toFixed(1)} / 5 rating</span>}
          <span>Discovery only</span>
        </div>
      </div>
    </Button>
  );
}
