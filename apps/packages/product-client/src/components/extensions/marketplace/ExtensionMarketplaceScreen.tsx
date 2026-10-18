import { Button } from "#product/primitives/Button";
import { Input } from "#product/primitives/Input";
import { Select } from "#product/primitives/Select";
import { Badge } from "#product/primitives/Badge";
import { EXTENSION_CATEGORIES, EXTENSION_PAGE_SIZE } from "#product/config/extensions/marketplace";
import { EXTENSION_MARKETPLACE_COPY as copy } from "#product/copy/extensions/marketplace";
import { useExtensionMarketplace } from "#product/hooks/extensions/ui/use-extension-marketplace";
import type { ExtensionSortBy } from "#product/lib/domain/extensions/extension-models";
import { ExtensionCard } from "./ExtensionCard";
import { ExtensionDetails } from "./ExtensionDetails";
import { ExtensionRegistryFailure } from "./ExtensionRegistryFailure";
import { InstalledExtensions } from "./InstalledExtensions";

export function ExtensionMarketplaceScreen({ appColorsAvailable = false }: { appColorsAvailable?: boolean }) {
  const state = useExtensionMarketplace();
  const { search } = state;
  return (
    <main className="flex min-h-0 min-w-0 flex-1 flex-col pt-[46px] text-foreground" data-telemetry-block>
      <header className="shrink-0 space-y-3 border-b border-border px-5 py-4">
        <div className="flex flex-wrap items-center gap-3"><h1 className="text-heading font-semibold">{copy.title}</h1><Badge>{copy.preview}</Badge></div>
        <p className="text-ui text-muted-foreground">{copy.subtitle}</p>
      </header>
      {state.selected ? <ExtensionDetails key={`${state.selected.namespace}.${state.selected.name}`} extensionRef={state.selected} onBack={() => state.setSelected(null)} />
        : <>
          <div className="shrink-0 space-y-3 border-b border-border p-5">
            <div className="flex gap-1">
              <Button variant={!state.installed ? "secondary" : "ghost"} aria-pressed={!state.installed} onClick={() => state.setInstalled(false)}>Discover</Button>
              <Button ref={state.installedTabRef} variant={state.installed ? "secondary" : "ghost"} aria-pressed={state.installed} onClick={() => state.setInstalled(true)}>Installed</Button>
            </div>
            {!state.installed && <>
              <Input ref={state.searchInputRef} type="search" aria-label="Search extensions" placeholder="Search by name, description, or publisher" value={state.filters.query} onChange={(event) => state.changeFilters({ query: event.target.value })} />
              <div className="grid gap-3 sm:grid-cols-2">
                <label className="space-y-1 text-ui-sm text-muted-foreground">Category<Select value={state.filters.category} onChange={(event) => state.changeFilters({ category: event.target.value })}><option value="">All categories</option>{EXTENSION_CATEGORIES.map((category) => <option key={category} value={category}>{category}</option>)}</Select></label>
                <label className="space-y-1 text-ui-sm text-muted-foreground">Sort by<Select value={state.filters.sortBy} onChange={(event) => state.changeFilters({ sortBy: event.target.value as ExtensionSortBy })}><option value="relevance">Relevance</option><option value="downloadCount">Popularity</option><option value="rating">Rating</option><option value="timestamp">Recently updated</option></Select></label>
              </div>
            </>}
          </div>
          <div className="min-h-0 flex-1 overflow-auto">
            {state.installed ? <InstalledExtensions appColorsAvailable={appColorsAvailable} onSelect={state.setSelected} />
              : search.isError ? <ExtensionRegistryFailure error={search.error} onRetry={() => { void search.refetch(); }} />
                : search.isPending ? <p role="status" className="p-5 text-ui text-muted-foreground">{copy.loading}</p>
                  : search.data.extensions.length === 0 ? <div className="p-5"><h2 className="text-ui font-semibold">{copy.empty}</h2><p className="mt-2 text-ui text-muted-foreground">{copy.emptyHint}</p></div>
                    : <>
                      <p role="status" className="px-5 pt-4 text-ui-sm text-muted-foreground">{search.data.totalSize.toLocaleString()} results</p>
                      <div className="grid gap-1 p-2 xl:grid-cols-2">{search.data.extensions.map((extension) => <ExtensionCard key={`${extension.id}@${extension.version}`} extension={extension} onSelect={() => state.setSelected(extension.ref)} />)}</div>
                    </>}
          </div>
          {!state.installed && <footer className="flex shrink-0 items-center justify-between gap-3 border-t border-border p-4 text-ui-sm text-muted-foreground">
            <Button variant="secondary" disabled={state.offset === 0 || search.isFetching} onClick={() => state.changePage(Math.max(0, state.offset - EXTENSION_PAGE_SIZE))}>Previous page</Button>
            <span>Page {Math.floor(state.offset / EXTENSION_PAGE_SIZE) + 1}</span>
            <Button variant="secondary" disabled={!state.hasNextPage || search.isFetching} onClick={() => state.changePage(state.offset + EXTENSION_PAGE_SIZE)}>Next page</Button>
          </footer>}
        </>}
    </main>
  );
}
