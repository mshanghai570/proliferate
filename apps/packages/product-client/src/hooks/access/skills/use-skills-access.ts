import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { resolveRuntimeCacheScopeKey, resolveRuntimeConnection, useAnyHarnessRuntimeContext } from "@anyharness/sdk-react";
import type { ImportSkillRequest, SetSkillEnabledRequest } from "@anyharness/sdk";
import { importSkill, listSkills, removeSkill, setSkillEnabled } from "#product/lib/access/anyharness/skills";

type Action = { kind: "import"; request: ImportSkillRequest } | { kind: "enable"; id: string; request: SetSkillEnabledRequest } | { kind: "remove"; id: string };
export function useSkillsAccess() {
  const runtime = useAnyHarnessRuntimeContext();
  const available = !!runtime.runtimeUrl?.trim();
  const connection = () => resolveRuntimeConnection(runtime);
  const queryKey = ["anyharness", resolveRuntimeCacheScopeKey(runtime), runtime.runtimeUrl, "skills"] as const;
  const client = useQueryClient();
  const query = useQuery({ queryKey, enabled: available,
    queryFn: ({ signal }) => listSkills(connection(), signal), retry: false,
  });
  const mutation = useMutation<unknown, Error, Action>({
    mutationFn: (action: Action) => {
      if (action.kind === "import") return importSkill(connection(), action.request);
      if (action.kind === "enable") return setSkillEnabled(connection(), action.id, action.request);
      return removeSkill(connection(), action.id);
    },
    onSettled: () => client.invalidateQueries({ queryKey }),
    retry: false,
  });
  return { query, mutation, available, scope: JSON.stringify(queryKey) };
}
