import { Button } from "#product/primitives/Button";
import { Badge } from "#product/primitives/Badge";
import { ArrowLeft } from "#product/primitives/icons/core";
import type { ExtensionRef } from "#product/lib/domain/extensions/extension-models";
import { resolveExtensionContentUrl } from "#product/lib/domain/extensions/extension-content";
import { useExtensionDetail, type ExtensionDetailTab } from "#product/hooks/extensions/ui/use-extension-detail";
import { useExtensionInstallation } from "#product/hooks/access/extensions/use-extension-installation";
import { EXTENSION_MARKETPLACE_COPY as copy } from "#product/copy/extensions/marketplace";
import { ExtensionRegistryFailure } from "./ExtensionRegistryFailure";
import { ExtensionDocument } from "./ExtensionDocument";

const detailTabs: ExtensionDetailTab[] = ["About", "Changelog", "Versions", "Capabilities"];

export function ExtensionDetails({ extensionRef, onBack }: { extensionRef: ExtensionRef; onBack: () => void }) {
  const { tab, setTab, metadata, manifest, compatibility, headingRef } = useExtensionDetail(extensionRef);
  const { install, runtimeAvailable } = useExtensionInstallation();
  const extension = metadata.data;
  const repository = resolveExtensionContentUrl(extension?.repository);
  const homepage = resolveExtensionContentUrl(extension?.homepage);
  return (
    <section aria-label="Extension details" className="flex min-h-0 min-w-0 flex-1 flex-col overflow-auto">
      <div className="px-5 pt-4">
        <Button variant="ghost" onClick={onBack}><ArrowLeft className="size-4" aria-hidden />Back to extensions</Button>
      </div>
      {metadata.isError ? <ExtensionRegistryFailure error={metadata.error} onRetry={() => { void metadata.refetch(); }} />
        : !extension ? <p role="status" className="p-5 text-ui text-muted-foreground">Loading extension…</p>
        : <>
          <header className="space-y-3 p-5">
            <div className="flex flex-wrap items-center gap-2">
              <h1 ref={headingRef} tabIndex={-1} className="text-xl font-semibold text-foreground">{extension.displayName}</h1>
              {extension.verified && <Badge>Verified namespace</Badge>}
              {extension.deprecated && <Badge tone="warning">Deprecated</Badge>}
            </div>
            <p className="text-ui text-muted-foreground">{extension.namespaceDisplayName ?? extension.ref.namespace} · {extension.version}</p>
            <p className="text-ui text-foreground">{extension.description}</p>
            <div className="flex flex-wrap items-center gap-3 text-ui-sm text-muted-foreground">
              {extension.downloadCount !== null && <span>{extension.downloadCount.toLocaleString()} downloads</span>}
              {extension.averageRating !== null && <span>{extension.averageRating.toFixed(1)} / 5 rating</span>}
              {!runtimeAvailable && <Badge tone="warning">Unsupported in this build</Badge>}
            </div>
            <Button 
              disabled={!runtimeAvailable || install.isPending} 
              onClick={() => { install.mutate({ ...extension.ref, version: extension.version }); }}
            >
              {install.isPending ? "Installing..." : copy.install}
            </Button>
            {!runtimeAvailable && <p id="extension-install-status" className="max-w-2xl text-ui text-muted-foreground">{copy.installUnavailable}</p>}
            {install.isSuccess && <p role="status" className="max-w-2xl text-ui text-muted-foreground">{copy.installSuccess}</p>}
            {install.isError && <p role="alert" className="max-w-2xl text-ui text-destructive">{copy.installFailure}</p>}
            {!extension.verified && <p className="max-w-2xl text-ui-sm text-muted-foreground">{copy.unverified}</p>}
          </header>
          <div className="flex flex-wrap gap-1 border-y border-border px-5 py-2" aria-label="Extension sections">
            {detailTabs.map((item) => <Button key={item} variant={tab === item ? "secondary" : "ghost"} aria-pressed={tab === item} onClick={() => setTab(item)}>{item}</Button>)}
          </div>
          <div className="grid min-w-0 flex-1 lg:grid-cols-[minmax(0,1fr)_240px]">
            <div className="min-w-0">
              {tab === "About" && <ExtensionDocument url={extension.fileUrls.readme} />}
              {tab === "Changelog" && <ExtensionDocument url={extension.fileUrls.changelog} />}
              {tab === "Versions" && <div className="space-y-3 p-5 text-ui">
                <h2 className="font-semibold">Version history</h2>
                {extension.allVersions.length === 0 ? <p className="text-muted-foreground">No version history provided.</p>
                  : <ul className="divide-y divide-border">{extension.allVersions.map((version) => <li key={version.version} className="flex gap-3 py-3"><span>{version.version}</span>{version.latest && <Badge>Latest</Badge>}</li>)}</ul>}
              </div>}
              {tab === "Capabilities" && <div className="space-y-4 p-5 text-ui">
                <p>{copy.noHost}</p>
                {manifest.isError ? <ExtensionRegistryFailure error={manifest.error} onRetry={() => { void manifest.refetch(); }} />
                  : !extension.fileUrls.manifest ? <p className="text-muted-foreground">The registry has not provided a manifest. Capabilities are unknown.</p>
                    : !manifest.data ? <p role="status">Inspecting manifest…</p>
                      : <>
                        <h2 className="font-semibold">Declared capabilities</h2>
                        <ul className="list-disc space-y-1 pl-5">{compatibility?.unsupportedCapabilities.map((capability) => <li key={capability}>{capability}</li>)}</ul>
                        <p className="text-muted-foreground">Requires VS Code API: {manifest.data.engineRequirement ?? "Not declared"}</p>
                        {(manifest.data.hasMainEntrypoint || manifest.data.hasBrowserEntrypoint) && <p>This extension contains executable code. Proliferate does not activate it.</p>}
                        <p className="text-muted-foreground">Manifest declarations describe capabilities, not a sandbox permission grant.</p>
                      </>}
              </div>}
            </div>
            <aside aria-label="Extension metadata" className="space-y-5 border-t border-border p-5 text-ui lg:border-l lg:border-t-0">
              <div><h2 className="mb-2 font-semibold">Publisher</h2><p className="break-words text-muted-foreground">{extension.publisher?.fullName ?? extension.publisher?.loginName ?? extension.ref.namespace}</p></div>
              <div><h2 className="mb-2 font-semibold">Dependencies</h2>
                <ul className="space-y-1 break-words text-muted-foreground">{extension.dependencies.length ? extension.dependencies.map((id) => <li key={id}>{id}</li>) : <li>None declared by the registry</li>}</ul>
              </div>
              {extension.bundledExtensions.length > 0 && <div><h2 className="mb-2 font-semibold">Extension pack</h2><ul className="space-y-1 break-words text-muted-foreground">{extension.bundledExtensions.map((id) => <li key={id}>{id}</li>)}</ul></div>}
              <div><h2 className="mb-2 font-semibold">Requirements</h2><p className="break-words text-muted-foreground">VS Code API: {extension.engines.vscode ?? "Not declared"}</p><p className="text-muted-foreground">Platform: {extension.targetPlatform ?? "Not specified"}</p></div>
              <div><h2 className="mb-2 font-semibold">Categories</h2><p className="text-muted-foreground">{extension.categories.join(", ") || "Not provided"}</p></div>
              <div><h2 className="mb-2 font-semibold">License</h2><p className="text-muted-foreground">{extension.license ?? "Not provided"}</p></div>
              <div className="flex flex-col gap-2">
                {repository && <a href={repository} target="_blank" rel="noopener noreferrer" className="text-primary underline">Repository</a>}
                {homepage && <a href={homepage} target="_blank" rel="noopener noreferrer" className="text-primary underline">Publisher website</a>}
              </div>
            </aside>
          </div>
        </>}
    </section>
  );
}
