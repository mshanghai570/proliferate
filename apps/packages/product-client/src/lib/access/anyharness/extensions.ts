import type { AnyHarnessRequestOptions, ExtensionPackageIdentity } from "@anyharness/sdk";
import { getAnyHarnessClient, type AnyHarnessClientConnection } from "@anyharness/sdk-react";

export function listInstalledExtensions(connection: AnyHarnessClientConnection, options?: AnyHarnessRequestOptions) {
  return getAnyHarnessClient(connection).extensions.list(options);
}

export function listExtensionColorThemes(connection: AnyHarnessClientConnection, identity: ExtensionPackageIdentity, options?: AnyHarnessRequestOptions) {
  return getAnyHarnessClient(connection).extensions.colorThemes(identity, options);
}

export function uninstallRegisteredExtension(connection: AnyHarnessClientConnection, identity: ExtensionPackageIdentity) {
  return getAnyHarnessClient(connection).extensions.uninstall(identity);
}

export function installRegisteredExtension(connection: AnyHarnessClientConnection, identity: ExtensionPackageIdentity, options?: AnyHarnessRequestOptions) {
  return getAnyHarnessClient(connection).extensions.install(identity, options);
}
