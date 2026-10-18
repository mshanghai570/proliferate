import { useId } from "react";
import type { ExtensionPackageIdentity } from "@anyharness/sdk";
import { Button } from "#product/primitives/Button";
import { useInstalledExtensionSyntax } from "#product/hooks/extensions/ui/use-installed-extension-syntax";

export function InstalledExtensionSyntax({ identity }: { identity: ExtensionPackageIdentity }) {
  const state = useInstalledExtensionSyntax(identity);
  const id = useId();
  return <div className="space-y-2">
    <Button variant="secondary" aria-expanded={state.open} aria-controls={id} onClick={() => state.setOpen(!state.open)}>Syntax themes</Button>
    {state.open && <div id={id} className="space-y-2">
      <p className="text-ui-sm text-muted-foreground">Apply syntax foreground colors and matching app colors. Covers source views, code blocks, and diffs. Reset syntax colors in Appearance.</p>
      {state.query.isPending ? <p role="status" className="text-ui-sm">Loading syntax themes…</p>
        : state.query.isError ? <div role="alert"><p className="text-ui-sm">Syntax themes are unavailable. Check the runtime connection or update the runtime, then retry.</p><Button variant="secondary" onClick={() => { void state.query.refetch(); }}>Retry</Button></div>
          : <>
            {!state.themes.length && <p className="text-ui-sm text-muted-foreground">This package declares no syntax themes.</p>}
            {state.themes.map(({ label, syntax, colors }, index) => <div key={index} className="flex flex-wrap items-center gap-2 text-ui-sm">
              <span>{label}</span>
              {syntax && colors ? <Button variant="secondary" aria-label={`Use ${label} syntax theme`} onClick={() => state.apply(syntax, colors)}>Use syntax theme</Button>
                : <span className="text-muted-foreground">Requires a standalone JSON theme with readable app colors and at most 256 foreground rules. Includes and external token files are unsupported.</span>}
            </div>)}
            {state.query.data?.truncated && <p className="text-ui-sm text-muted-foreground">Showing the first 32 declared themes.</p>}
          </>}
      {state.applied && <p role="status" className="text-ui-sm">Applied {state.applied}. This theme copy survives package removal.</p>}
    </div>}
  </div>;
}
