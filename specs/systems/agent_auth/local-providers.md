# Local model API providers

Status: current

The macOS app's LLM providers page can save OpenAI-compatible connections in a
runtime-local encrypted vault. This local mode needs no Proliferate account or
control-plane service. It supplies API models to local OpenCode processes;
other harnesses retain their own configuration.

## Ownership and precedence

The [local provider domain](../../../anyharness/crates/anyharness-lib/src/domains/agents/local_providers/mod.rs)
owns local choices. Managed choices continue to be owned by the server. A
matching applied managed auth document takes precedence over the local vault,
even when its selected sources are unsatisfiable. Local providers cannot repair
or override that managed selection. When no matching document exists, OpenCode
launch, readiness, and probe resolution use enabled local provider sources.
Other harnesses never read this vault.

Local sources use the existing provider-config render path, stable provider IDs,
and dedicated key environment variables. Keyless connections explicitly set an
empty API key to avoid ambient-key fallback. Negative local materialization
revisions avoid collisions with the managed document's nonnegative revisions.
The encrypted catalog fingerprint participates in OpenCode's launch-options
basis; changes poke that harness's observation engine. Model membership remains
target-observed. Saving does not verify connectivity or tool support.

## Storage

The vault lives at `runtime_home/agent-auth/local-providers/providers.json`.
Only identity, name, enabled state, and model count are plaintext metadata.
Endpoint, models, API format, and key are encrypted with the existing runtime
data key and AES-256-GCM-SIV format. The
[shared cipher](../../../anyharness/crates/anyharness-lib/src/persistence/data_cipher.rs)
also serves MCP bindings without changing their ciphertext format or key.
The Mac host's existing keychain provisioning supplies `ANYHARNESS_DATA_KEY`.

Catalog writes hold an exclusive file lock, create private temporary files,
sync, rename atomically, and sync their parent. Unix files use mode 0600.
Catalog reads are bounded to 4 MiB; there are at most 32 providers, with at most
100 model IDs and 64 KiB of configuration per provider. URLs must be HTTP(S)
without credentials, queries, fragments, or substitutions. Keys cannot contain
line breaks. Invalid enabled ciphertext refuses a launch; it is never skipped.
Disabling/removing a valid catalog entry needs no decryption key.

Credentials are write-only. They are not returned in inventory, placed in UI
preferences or mutation caches, or included in server auth-state delivery.
Running processes retain their credentials until they stop. Editing credentials
currently requires removing and adding a connection.

## API and client

Runtime owner authentication protects these routes; workspace JWTs cannot use
them. Inputs and failures do not echo credentials.

- `GET /v1/agent-auth/local-providers`: metadata, encryption availability, and
  whether managed authentication currently pauses local choices.
- `POST /v1/agent-auth/local-providers`: validate, encrypt, and enable a connection.
- `PATCH /v1/agent-auth/local-providers/{id}`: enable or disable one connection.
- `DELETE /v1/agent-auth/local-providers/{id}`: remove one connection.

The [HTTP mapper](../../../anyharness/crates/anyharness-lib/src/api/http/local_providers.rs)
translates wire DTOs to domain values. SDK agent-auth methods expose the routes.
Settings access scopes caches by runtime and actor, updates safe metadata after
writes, and invalidates inventory. A keyed editor discards drafts across scopes.
Managed mode blocks new enables; cleanup remains available.

## Evidence and observability

The bounded `anyharness.local_provider.changed` event carries only create,
toggle, or remove. It excludes names, paths, model IDs, endpoints, and keys.

[Domain tests](../../../anyharness/crates/anyharness-lib/src/domains/agents/local_providers/tests.rs)
cover encrypted storage, credential redaction, keyless configuration, invalid
inputs, capacity, corruption, missing keys, and managed precedence.
[Real-router tests](../../../anyharness/crates/anyharness-lib/src/api/local_provider_tests.rs)
cover owner authentication, metadata-only responses, launch credential delivery
without server state, and disabling/removal. SDK and settings tests cover route
encoding, write-only drafts, duplicate submits, errors, and scope changes.
The Intel preview workflow runs these checks on GitHub before publishing its
artifact; generated API contracts come from the actual built runtime.

Owner: [Agent auth](README.md).
