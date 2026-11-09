import { useState } from "react";
import { useProjectRetrievalAccess } from "#product/hooks/access/retrieval/use-project-retrieval-access";
import { formatProjectContext } from "#product/lib/domain/chat/project-context";
export function useProjectContextPicker(workspace: string, onAdd: (text: string) => void) {
  const access = useProjectRetrievalAccess(workspace);
  const [query, setQuery] = useState("");
  const [selected, setSelected] = useState<number[]>([]);
  const [error, setError] = useState<string | null>(null);
  const busy = access.refresh.isPending || access.clear.isPending || access.search.isPending;
  const reset = () => { setSelected([]); setError(null); access.search.reset(); };
  const index = () => { reset(); access.clear.reset(); access.refresh.mutate(); };
  const clear = () => { reset(); access.refresh.reset(); access.clear.mutate(); };
  const search = () => { reset(); access.refresh.reset(); access.clear.reset(); access.search.mutate(query); };
  const excerpts = access.search.data?.workspaceId === workspace ? access.search.data.excerpts : [];
  const add = () => {
    try {
      const text = formatProjectContext(excerpts.filter((_, index) => selected.includes(index)));
      onAdd(text); return true;
    } catch (cause) { setError(cause instanceof Error ? cause.message : "Choose excerpts again."); return false; }
  };
  return { access, query, selected, excerpts, busy, error, index, clear, search, add,
    editQuery: (text: string) => { reset(); setQuery(text); },
    toggle: (index: number) => { setError(null); setSelected((current) => current.includes(index) ? current.filter((value) => value !== index) : [...current, index]); },
  };
}
