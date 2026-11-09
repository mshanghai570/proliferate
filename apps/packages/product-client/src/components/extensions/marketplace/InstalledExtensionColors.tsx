import { useId } from "react";
import type { ExtensionPackageIdentity } from "@anyharness/sdk";
import { Button } from "#product/primitives/Button";
import { useInstalledExtensionColors } from "#product/hooks/extensions/ui/use-installed-extension-colors";

export function InstalledExtensionColors({ identity }: { identity: ExtensionPackageIdentity }) {
  const state = useInstalledExtensionColors(identity);
  const { open, query, applied } = state;
  const contentId = useId();
  return <div className="space-y-2">
    <Button variant="secondary" aria-expanded={open} aria-controls={contentId} onClick={() => state.setOpen(!open)}>App color themes</Button>
    {open && <div id={contentId} className="space-y-2">
      <p className="text-ui-sm text-muted-foreground">Use core app colors from a declared theme. Syntax colors are separate. Reset the imported colors in Appearance settings.</p>
      {query.isPending ? <p role="status" className="text-ui-sm">Loading color themes…</p>
        : query.isError ? <div role="alert"><p className="text-ui-sm">Color themes are unavailable. Refresh Installed and retry; older runtimes may need an update.</p><Button variant="secondary" onClick={() => { void query.refetch(); }}>Retry</Button></div>
          : <>
            {!state.themes.length && <p className="text-ui-sm text-muted-foreground">This package declares no color themes.</p>}
            {state.themes.map(({ label, colors }, index) => {
              return <div key={index} className="flex flex-wrap items-center gap-2 text-ui-sm">
                <span>{label}</span>
                {colors ? <Button variant="secondary" aria-label={`Use ${label} app colors`} onClick={() => state.apply(colors)}>Use app colors</Button> : <span className="text-muted-foreground">Requires readable six-digit hex colors in a standalone JSON theme.</span>}
              </div>;
            })}
            {query.data?.truncated && <p className="text-ui-sm text-muted-foreground">Showing the first 32 declared themes.</p>}
          </>}
      {applied && <p role="status" className="text-ui-sm">Applied {applied}. The color copy remains available if the package is removed.</p>}
    </div>}
  </div>;
}
