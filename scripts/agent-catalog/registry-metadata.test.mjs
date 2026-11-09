import assert from "node:assert/strict";
import test from "node:test";
import { catalogForProviderConfigChange } from "./registry-metadata.mjs";

const previousRegistry = { registryVersion: "2026-08-20.1", agents: [{ kind: "opencode", launch: { defaultArgs: ["acp"] } }] };
const registry = { ...previousRegistry, registryVersion: "2026-10-16.1", agents: [{ ...previousRegistry.agents[0], providerConfig: [{ kind: "openai_compatible" }] }] };
const catalog = { schemaVersion: 2, catalogVersion: "2026-08-20.1", probedAgainst: { registryVersion: previousRegistry.registryVersion }, agents: [{ kind: "opencode", harness: { agentProcess: { version: "1.18.3", source: { sha256: "pinned" } } }, session: { presentationModels: [{ id: "known-model" }] } }] };

test("provider-config metadata regeneration preserves every existing agent and pin", () => {
  const next = catalogForProviderConfigChange(catalog, previousRegistry, registry, new Date("2026-10-16T12:00:00Z"));
  assert.deepEqual(next.agents, catalog.agents);
  assert.deepEqual(next.probedAgainst, { registryVersion: registry.registryVersion });
  assert.equal(next.catalogVersion, "2026-10-16.1");
});

test("cannot use metadata regeneration to change executable or auth declarations", () => {
  for (const change of [{ launch: { defaultArgs: ["different"] } }, { authCardinality: "multi" }, { auth: { slots: [] } }]) {
    const changed = { ...registry, agents: [{ ...registry.agents[0], ...change }] };
    assert.throws(() => catalogForProviderConfigChange(catalog, previousRegistry, changed), /providerConfig/);
  }
});

test("requires the paired previous registry and a newer registry version", () => {
  assert.throws(() => catalogForProviderConfigChange(catalog, { ...previousRegistry, registryVersion: "2026-08-21.1" }, registry), /paired/);
  assert.throws(() => catalogForProviderConfigChange(catalog, previousRegistry, { ...registry, registryVersion: previousRegistry.registryVersion }), /increase/);
});
