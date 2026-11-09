import { useEffect, useRef, useState } from "react";
import { useDebouncedValue } from "#product/hooks/ui/timing/use-debounced-value";
import { useExtensionSearch } from "#product/hooks/access/extensions/use-extension-registry";
import { EXTENSION_PAGE_SIZE } from "#product/config/extensions/marketplace";
import type { ExtensionRef, ExtensionSortBy } from "#product/lib/domain/extensions/extension-models";

export function useExtensionMarketplace() {
  const [filters, setFilters] = useState({ query: "", category: "", sortBy: "downloadCount" as ExtensionSortBy });
  const debouncedFilters = useDebouncedValue(filters, 250);
  const [page, setPage] = useState({ filters: debouncedFilters, offset: 0 });
  // A new filter starts on page one without an effect or an intermediate stale page.
  const offset = page.filters === debouncedFilters ? page.offset : 0;
  const [selected, setSelected] = useState<ExtensionRef | null>(null);
  const searchInputRef = useRef<HTMLInputElement>(null);
  const installedTabRef = useRef<HTMLButtonElement>(null);
  const [installed, setInstalled] = useState(false);
  const previousSelection = useRef(selected);
  useEffect(() => {
    if (previousSelection.current && !selected) {
      (installed ? installedTabRef.current : searchInputRef.current)?.focus();
    }
    previousSelection.current = selected;
  }, [selected, installed]);
  const search = useExtensionSearch({
    query: debouncedFilters.query.trim(), category: debouncedFilters.category || undefined,
    sortBy: debouncedFilters.sortBy, sortOrder: "desc", size: EXTENSION_PAGE_SIZE, offset,
  });
  const changeFilters = (patch: Partial<typeof filters>) => setFilters((current) => ({ ...current, ...patch }));
  return {
    filters, changeFilters, offset, search, selected, setSelected, searchInputRef, installedTabRef, installed, setInstalled,
    changePage: (nextOffset: number) => setPage({ filters: debouncedFilters, offset: nextOffset }),
    hasNextPage: !!search.data && search.data.extensions.length > 0 && offset + search.data.extensions.length < search.data.totalSize,
  };
}
