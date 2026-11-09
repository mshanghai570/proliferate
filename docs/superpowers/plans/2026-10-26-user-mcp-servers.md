# User-Configured MCP Servers Implementation Plan

> **For agentic workers:** Implement the frozen delivery specification task by task. Preserve all pre-existing worktree changes; do not commit unrelated files.

**Goal:** Let a runtime user manage stdio and HTTP MCP servers in Settings, seal attached secrets at rest, and select servers by user default or repository override for future session launches.

**Architecture:** AnyHarness owns the durable user-server catalog, admission, secret sealing, API, and a session launch extension. The existing session MCP binding assembly merges those servers with product MCPs; ProductClient accesses the runtime API through the generated SDK and exposes controls in Settings → Agents → MCP servers.

**Tech Stack:** Rust, SQLite/rusqlite, Axum, AnyHarness OpenAPI/code generation, TypeScript, React, TanStack Query, Vitest.

**Spec:** `delivery/daily-driver/delivery-spec-daily-driver-pr1-user-mcp-servers.md`

## Global Constraints

- Seal secret values with the existing `SessionDataCipher`; refuse writes that contain secrets when the data key is unavailable.
- Stdio admission is user-authorized but bounded; reject control characters, limit server names to 100 printable UTF-8 bytes, command arguments to 32, and env/header credentials to 16.
- Exclude secret values and decrypted configurations from API responses, logs, summaries, and transcripts.
- Apply enablement changes only to subsequent launches; an explicit repository boolean overrides the user default and `null` resets inheritance.
- `InternalOnly` sessions receive no user-configured MCP servers.
- Keep server management runtime-local; do not add marketplace distribution, organization policy, cloud synchronization, per-session attachment, or OAuth.
- Preserve existing worktree changes and generated-code ownership; regenerate OpenAPI/SDK artifacts through their owning commands.

---

### Task 1: Complete MCP admission, storage, and launch resolution

**Files:**
- Modify: `anyharness/crates/anyharness-contract/src/v1/mcp_servers.rs`
- Modify: `anyharness/crates/anyharness-contract/src/v1/mod.rs`
- Modify: `anyharness/crates/anyharness-lib/src/domains/mcp_servers/admission.rs`
- Modify: `anyharness/crates/anyharness-lib/src/domains/mcp_servers/extension.rs`
- Modify: `anyharness/crates/anyharness-lib/src/domains/mcp_servers/manager.rs`
- Modify: `anyharness/crates/anyharness-lib/src/domains/mcp_servers/model.rs`
- Modify: `anyharness/crates/anyharness-lib/src/domains/mcp_servers/mod.rs`
- Modify: `anyharness/crates/anyharness-lib/src/domains/mcp_servers/store.rs`
- Modify: `anyharness/crates/anyharness-lib/src/domains/mod.rs`
- Modify: `anyharness/crates/anyharness-lib/src/persistence/migrations.rs`
- Create: `anyharness/crates/anyharness-lib/src/persistence/sql/0078_user_mcp_servers.sql`
- Modify: `anyharness/crates/anyharness-lib/src/app/mod.rs`
- Test: unit tests colocated with the MCP domain and persistence migration/schema tests

**Interfaces:**
- `UserMcpServerManager` owns list, create, enablement, and removal operations over `Db` and optional `SessionDataCipher`.
- `admit_create(server_name, description, config)` validates a `McpServerConfig` and returns `AdmittedMcpServer`.
- `UserMcpServersLaunchExtension::resolve_launch_extras` produces selected `SessionMcpServer` values and redacted `SessionMcpBindingSummary` values for the launch's repository root.

- [x] **Step 1: Verify the current in-progress source against Task 1 requirements.** Read the files listed above and compare each implementation detail to the frozen spec; retain sound existing changes and add tests for uncovered behavior rather than replacing working code.
- [x] **Step 2: Add or complete storage tests.** Prove SQLite reopen persistence, unique server names, the 32-record capacity, malformed stored JSON failure, transport decoding, and secret-envelope-at-rest behavior.
- [x] **Step 3: Add or complete admission and selection tests.** Prove the 32-argument, 16-env/header, and bounded-string limits; reject control characters and non-HTTP(S) URLs; prove user-default inheritance, repository override/reset, and that deleting or changing a record affects only future launches.
- [x] **Step 4: Add or complete launch-extension tests.** Prove stdio and HTTP launch payloads decrypt credentials only at launch, `InternalOnly` excludes all user servers, summaries contain names but not values, missing data keys produce the specified not-applied reason, and product/user servers merge in the required order.
- [x] **Step 5: Run the focused Rust tests.** Run `cargo test -p anyharness-lib mcp_servers --lib` from `anyharness/`; expected: all matching domain and extension tests pass. If this selector omits colocated tests due module naming, run `cargo test -p anyharness-lib --lib` and report the exact outcome.

### Task 2: Finish the authenticated runtime API and generated contract

**Files:**
- Modify: `anyharness/crates/anyharness-lib/src/api/http/mcp_servers.rs`
- Modify: `anyharness/crates/anyharness-lib/src/api/http/mod.rs`
- Modify: `anyharness/crates/anyharness-lib/src/api/router.rs`
- Modify: `anyharness/crates/anyharness-lib/src/api/openapi.rs`
- Modify: `anyharness/crates/anyharness-lib/src/api/openapi_tests.rs`
- Modify: `anyharness/crates/anyharness-lib/src/persistence/migrations.rs`
- Modify: `anyharness/crates/anyharness-contract/src/v1/mcp_servers.rs`
- Regenerate: `anyharness/sdk/generated/openapi.json`, `anyharness/sdk/src/generated/openapi.ts`, and `specs/areas/anyharness-db-schema.sql` with the owning generators
- Test: `anyharness/crates/anyharness-lib/src/api/http/mcp_servers_tests.rs` (or the established API test module for this route)

**Interfaces:**
- `GET /v1/mcp-servers` returns summaries only.
- `POST /v1/mcp-servers` accepts a server name, optional description, and tagged stdio/HTTP config.
- `PUT /v1/mcp-servers/{id}/enabled` accepts either a user-default boolean or a repository override containing `repo_root_id` and `enabled`; `enabled: null` resets only that repository override.
- `DELETE /v1/mcp-servers/{id}` removes the record and all its selection state.

- [x] **Step 1: Add failing route tests** for runtime authentication, route registration, invalid config bounds, missing data-key errors, repository existence checks, workspace-scoped management denial, the exact tagged user-default/repository-override request shapes, and secret-free response bodies.
- [x] **Step 2: Implement route behavior** by keeping transport translation in `api/http/mcp_servers.rs`, using typed bounded error codes, and ensuring every management route follows the runtime bearer boundary and global-management scope rules.
- [x] **Step 3: Register contract types and routes in OpenAPI** and add assertions that all four endpoints and request/response schemas are represented.
- [x] **Step 4: Regenerate the SDK OpenAPI artifacts and database schema snapshot.** From `anyharness/sdk/`, run `pnpm generate`; from `anyharness/`, run `cargo test -p anyharness-lib update_anyharness_schema_snapshot -- --ignored`. Do not hand-edit generated files.
- [x] **Step 5: Run focused Rust API/OpenAPI tests.** Run `cargo test -p anyharness-lib mcp_servers --lib` and the relevant OpenAPI test selector from `anyharness/`; expected: route, auth, schema, and OpenAPI tests pass.

### Task 3: Add SDK access and the MCP Settings pane

**Files:**
- Create: `anyharness/sdk/src/client/mcp_servers.ts`
- Create: `anyharness/sdk/src/client/mcp_servers.test.ts`
- Create: `apps/packages/product-client/src/lib/access/anyharness/mcp-servers.ts`
- Create: `apps/packages/product-client/src/hooks/access/agents/use-mcp-servers-access.ts`
- Create: `apps/packages/product-client/src/components/settings/panes/agents/mcp/McpServersPane.tsx`
- Create: `apps/packages/product-client/src/components/settings/panes/agents/mcp/McpServersPane.test.tsx`
- Modify: `apps/packages/product-client/src/config/settings.ts`
- Modify: `apps/packages/product-client/src/components/settings/{sidebar/SettingsSidebar.tsx,screen/render-settings-section.tsx}`
- Create: `apps/packages/product-client/src/copy/settings/agents/mcp-servers-copy.ts`
- Test: SDK client tests and ProductClient MCP-pane tests

**Interfaces:**
- The SDK methods use generated request/response types and the existing runtime-client transport.
- The access hook exposes list state, connection availability, mutations, pending/error state, and cache invalidation for create/enable/remove.
- The pane manages one draft at a time, lists stdio/HTTP servers, edits user or repository enablement, and confirms deletion. Secret values are shown only while entering a new server and are never refetched.

- [x] **Step 1: Add SDK tests** for list/create/enable/remove URL, method, request serialization, and typed response handling; verify the generated OpenAPI types expose the wire contract.
- [x] **Step 2: Implement SDK and access layers** using existing AnyHarness runtime query/mutation patterns and runtime-scoped cache keys; add tests for cache invalidation and disconnected-runtime handling.
- [x] **Step 3: Register the Settings section and pane** under the existing Agents settings navigation, following the skills pane's scope selector and local-runtime behavior.
- [x] **Step 4: Implement the add form and server list.** Support stdio command/args/env pairs and HTTP URL/header pairs, render entered secrets as masked values without ever loading secret values, start records disabled, and provide user-default/repository-override controls plus confirm-before-delete.
- [x] **Step 5: Add component tests** covering both transports, create, enable/disable, repository override/reset, delete confirmation, error and retry states, disconnected-runtime copy, and absence of secret values in rendered output.
- [x] **Step 6: Run SDK and ProductClient tests.** From `anyharness/sdk/`, run `pnpm test -- src/client/mcp_servers.test.ts`; from the repository root, run `pnpm --filter @proliferate/product-client test -- src/components/settings/panes/agents/mcp/McpServersPane.test.tsx`; expected: both focused suites pass.

### Task 4: Update owning system documentation and verify the end-to-end gate

**Files:**
- Create: `specs/systems/harnesses/mcp-servers.md`
- Modify: `specs/systems/harnesses/README.md`
- Modify: `specs/systems/settings/README.md`
- Modify: `specs/areas/anyharness.md` only if its source-layout or generation instructions need a current-state update
- Test: relevant AnyHarness Rust tests, ProductClient tests, and repository documentation checks

- [x] **Step 1: Document current behavior** in `mcp-servers.md` using the skills spec's structure: ownership, selection, admission, secret storage, API, launch delivery, code map, observability, and proof.
- [x] **Step 2: Add the harnesses index entry and Settings pane row** without duplicating behavior descriptions in the routing docs.
- [x] **Step 3: Run documentation validation.** Run `python3 scripts/check_docs.py`; expected: exit code 0.
- [x] **Step 4: Run the targeted Rust and frontend suites together** and inspect the final diff to ensure unrelated existing changes remain untouched.
- [ ] **Step 5: Perform the product acceptance gate**: add stdio and HTTP servers with secrets in Settings, enable one for a repository, launch a session and verify its tools and binding summary, then verify a second repository without an override does not receive it. Confirm logs, summaries, transcripts, and API responses never contain secret values.
