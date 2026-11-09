import { createContext, useContext, useState, type ReactNode } from "react";
import { createOpenVsxRegistry } from "#product/lib/access/extensions/open-vsx-registry";
import type { ExtensionRegistry } from "#product/lib/domain/extensions/extension-models";

const ExtensionRegistryContext = createContext<ExtensionRegistry | null>(null);

export function ExtensionRegistryProvider({ children }: { children: ReactNode }) {
  const [registry] = useState(createOpenVsxRegistry);
  return <ExtensionRegistryContext.Provider value={registry}>{children}</ExtensionRegistryContext.Provider>;
}

export function useExtensionRegistry(): ExtensionRegistry {
  const registry = useContext(ExtensionRegistryContext);
  if (!registry) throw new Error("ExtensionRegistryProvider is required");
  return registry;
}
