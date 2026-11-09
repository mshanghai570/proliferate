import { isDeepStrictEqual } from "node:util";
import { compareVersions } from "./check-version-discipline.mjs";

function withoutProviderConfigs(registry) {
  const result = structuredClone(registry);
  delete result.registryVersion;
  for (const agent of result.agents ?? []) delete agent.providerConfig;
  return result;
}

/** Rebuild metadata when the only registry delta is providerConfig declarations. */
export function catalogForProviderConfigChange(catalog, previousRegistry, registry, now = new Date()) {
  if (!isDeepStrictEqual(withoutProviderConfigs(previousRegistry), withoutProviderConfigs(registry))) {
    throw new Error("Only providerConfig declarations may change; use the probe pipeline for other registry changes.");
  }
  if (catalog?.schemaVersion !== 2 || catalog.probedAgainst?.registryVersion !== previousRegistry.registryVersion) {
    throw new Error("The supplied previous registry must be paired with the bundled catalog.");
  }
  if (!(compareVersions(registry.registryVersion, previousRegistry.registryVersion) > 0)) {
    throw new Error("The registry version must increase for providerConfig changes.");
  }
  const today = now.toISOString().slice(0, 10);
  const [previousDay, previousRevision] = catalog.catalogVersion.split(".");
  const nextVersion = `${today}.${previousDay === today ? Number(previousRevision) + 1 : 1}`;
  if (!(compareVersions(nextVersion, catalog.catalogVersion) > 0)) {
    throw new Error("The catalog version must increase; check the generation clock.");
  }
  return {
    ...catalog,
    catalogVersion: nextVersion,
    probedAgainst: { registryVersion: registry.registryVersion },
    generatedAt: now.toISOString(),
  };
}
