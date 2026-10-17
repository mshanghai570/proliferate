import { ExtensionMarketplaceScreen } from "#product/components/extensions/marketplace/ExtensionMarketplaceScreen";
import { MainSidebarPageShell } from "#product/components/workspace/shell/screen/MainSidebarPageShell";
import { ExtensionRegistryProvider } from "#product/providers/ExtensionRegistryProvider";
import { isExtensionMarketplaceEnabled } from "#product/lib/domain/capabilities/extension-marketplace";
import { EXTENSION_MARKETPLACE_COPY as copy } from "#product/copy/extensions/marketplace";

export function ExtensionsPage() {
  return (
    <MainSidebarPageShell>
      {isExtensionMarketplaceEnabled()
        ? <ExtensionRegistryProvider><ExtensionMarketplaceScreen /></ExtensionRegistryProvider>
        : <main className="flex-1 p-5 pt-20 text-foreground"><h1 className="text-heading font-semibold">{copy.unavailable}</h1><p className="mt-3 text-ui text-muted-foreground">{copy.unavailableHint}</p></main>}
    </MainSidebarPageShell>
  );
}
