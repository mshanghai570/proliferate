import type { ExtensionRef } from "#product/lib/domain/extensions/extension-models";
import { Button } from "#product/primitives/Button";
import { Badge } from "#product/primitives/Badge";
import { EXTENSION_MANAGEMENT_COPY as copy } from "#product/copy/extensions/management";
import { useInstalledExtensions } from "#product/hooks/extensions/ui/use-installed-extensions";
import { InstalledExtensionColors } from "#product/components/extensions/marketplace/InstalledExtensionColors";

export function InstalledExtensions({ appColorsAvailable = false, onSelect }: {
  appColorsAvailable?: boolean;
  onSelect?: (extension: ExtensionRef) => void;
}) {
  const state = useInstalledExtensions();
  if (!state.runtimeAvailable) {
    return <p className="p-5 text-ui text-muted-foreground">{copy.disconnected}</p>;
  }
  return <section className="space-y-4 p-5" aria-label={copy.title}>
    <div className="flex items-center justify-between gap-3">
      <h2 className="text-ui font-semibold">{copy.title}</h2>
      <Button
        ref={state.refreshRef}
        variant="secondary"
        disabled={state.query.isFetching || state.uninstall.isPending}
        onClick={() => { void state.query.refetch(); }}
      >{copy.refresh}</Button>
    </div>
    <p className="text-ui text-muted-foreground">{copy.inactive}</p>
    {state.uninstallError && <p role="alert" className="text-ui text-destructive">{state.uninstallError}</p>}
    {state.queryError ? <div role="alert">
      <p className="text-ui">{state.queryError}</p>
      <Button variant="secondary" onClick={() => { void state.query.refetch(); }}>{copy.retry}</Button>
    </div>
      : state.query.isPending ? <p role="status" className="text-ui text-muted-foreground">{copy.loading}</p>
        : !state.query.data?.extensions.length ? <div><h3 className="text-ui font-semibold">{copy.empty}</h3><p className="mt-2 text-ui text-muted-foreground">{copy.emptyHint}</p></div>
          : <ul className="divide-y divide-border">{state.query.data.extensions.map((extension) => {
            const id = `${extension.namespace}.${extension.name}@${extension.version}`;
            const confirming = state.pending?.namespace === extension.namespace && state.pending.name === extension.name && state.pending.version === extension.version;
            return <li key={id} className="space-y-3 py-4">
              <div className="flex flex-wrap items-start justify-between gap-3">
                <div>
                  <h3 className="text-ui font-semibold">{extension.namespace}.{extension.name}</h3>
                  <p className="text-ui-sm text-muted-foreground">{extension.version}</p>
                </div>
                <Badge>{copy.states[extension.state] ?? copy.states.error}</Badge>
              </div>
              {extension.errorCode && <p className="text-ui text-destructive">{copy.packageErrors[extension.errorCode] ?? copy.packageErrors.package_invalid}</p>}
              {appColorsAvailable && extension.state === "installed" && <InstalledExtensionColors key={`${state.scope}:${id}`} identity={extension} />}
              <div className="flex flex-wrap gap-2">
                {onSelect && <Button
                  variant="secondary"
                  aria-label={`${copy.details} for ${extension.namespace}.${extension.name}`}
                  disabled={state.uninstall.isPending}
                  onClick={() => onSelect({ namespace: extension.namespace, name: extension.name })}
                >{copy.details}</Button>}
                <Button
                  variant="secondary"
                  aria-label={`Uninstall ${id}`}
                  disabled={state.uninstall.isPending || confirming}
                  onClick={(event) => state.requestUninstall(extension, event.currentTarget)}
                >{copy.uninstall}</Button>
              </div>
              {confirming && <div className="space-y-2">
                <p className="text-ui text-muted-foreground" aria-live="polite">{copy.confirmHint}</p>
                <div className="flex gap-2">
                  <Button
                    ref={state.confirmRef}
                    variant="secondary"
                    disabled={state.uninstall.isPending}
                    onClick={state.confirm}
                  >{state.uninstall.isPending ? copy.removing : copy.confirm}</Button>
                  <Button variant="ghost" disabled={state.uninstall.isPending} onClick={state.cancel}>{copy.cancel}</Button>
                </div>
              </div>}
            </li>;
          })}</ul>}
  </section>;
}
