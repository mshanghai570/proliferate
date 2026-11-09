import type { components } from "../generated/openapi.js";

export type InstalledExtensionSummary = components["schemas"]["InstalledExtensionSummary"];
export type ListInstalledExtensionsResponse = components["schemas"]["ListInstalledExtensionsResponse"];
export type ExtensionPackageIdentity = Pick<InstalledExtensionSummary, "namespace" | "name" | "version">;
