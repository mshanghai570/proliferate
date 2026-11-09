import type { ExtensionPackageIdentity, InstalledExtensionSummary, ListInstalledExtensionsResponse } from "../types/extensions.js";
import type { AnyHarnessRequestOptions, AnyHarnessTransport } from "./core.js";

export class ExtensionsClient {
  constructor(private readonly transport: AnyHarnessTransport) {}

  list(options?: AnyHarnessRequestOptions): Promise<ListInstalledExtensionsResponse> {
    return this.transport.get("/v1/extensions", options);
  }

  uninstall(identity: ExtensionPackageIdentity, options?: AnyHarnessRequestOptions): Promise<void> {
    return this.transport.delete(
      `/v1/extensions/${encodeURIComponent(identity.namespace)}/${encodeURIComponent(identity.name)}/${encodeURIComponent(identity.version)}`,
      options,
    );
  }

  install(identity: ExtensionPackageIdentity, options?: AnyHarnessRequestOptions): Promise<InstalledExtensionSummary> {
    return this.transport.post("/v1/extensions/install", identity, options);
  }
}
