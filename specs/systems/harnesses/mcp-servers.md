# User MCP Servers

User-configured MCP servers are runtime-local tool endpoints attached to future
agent launches. The [harnesses system](README.md) owns their definition,
admission, storage, selection, and launch resolution; [sessions](../sessions/README.md)
owns live session execution and [product MCP servers](../subagents/product-mcp-servers.md)
owns product-provided MCP behavior and protocol serving.

## Selection and launch

A user-managed server has one user default and optional repository-root
overrides. An explicit repository boolean wins; removing the override restores
the default, whose unset value means disabled. Settings resolves repository
choices using AnyHarness repo-root ids, so a repository's worktrees share its
selection. Changing or removing a server affects future launches only; an
already-running session keeps its launch configuration.

At launch, [`UserMcpServersLaunchExtension`](../../../anyharness/crates/anyharness-lib/src/domains/mcp_servers/extension.rs)
filters the durable catalog against the session's repository root. It returns
concrete stdio or HTTP server configurations as launch extras. The session MCP
binding assembler merges product servers first and user servers afterward,
and produces the corresponding binding summaries
([`assembly.rs`](../../../anyharness/crates/anyharness-lib/src/domains/sessions/mcp_bindings/assembly.rs)).
An `InternalOnly` session receives no user-configured servers.

## Admission and durable state

The runtime API accepts a stdio command with literal arguments and environment
credentials, or an HTTP(S) URL with header credentials. Admission is free for
commands the user enters, but bounded: at most 32 servers, 32 arguments, and
16 credentials per server; server names are at most 100 UTF-8 bytes. Bounded
strings reject control characters. HTTP URLs must use `http` or `https`.
[`admission.rs`](../../../anyharness/crates/anyharness-lib/src/domains/mcp_servers/admission.rs)
owns validation.

[`user_mcp_servers`](../../../anyharness/crates/anyharness-lib/src/persistence/sql/0078_user_mcp_servers.sql)
stores each server and its selection state in the runtime SQLite database.
[`UserMcpServerManager`](../../../anyharness/crates/anyharness-lib/src/domains/mcp_servers/manager.rs)
seals every credential value independently with the existing
`SessionDataCipher`; credential names remain available as metadata. Creating a
server with secrets fails with `MCP_SERVER_DATA_KEY_REQUIRED` when
`ANYHARNESS_DATA_KEY` is unavailable. The launch extension decrypts credentials
only while preparing a launch. Catalog or selection corruption fails the
operation rather than silently dropping a configured server.

## Public surface

All routes use runtime authentication and are unavailable to workspace-scoped
claims, because this catalog belongs to the runtime user rather than one
workspace.

| Route | Meaning |
| --- | --- |
| `GET /v1/mcp-servers` | List server metadata, credential names, and selection state. |
| `POST /v1/mcp-servers` | Create a server; new servers start disabled. |
| `PUT /v1/mcp-servers/{id}/enabled` | A JSON boolean sets the user default; `{ "repoRootId": string, "enabled": boolean \| null }` sets or resets one existing repo-root override. |
| `DELETE /v1/mcp-servers/{id}` | Remove the server and its selection state. |

[`mcp_servers.rs`](../../../anyharness/crates/anyharness-lib/src/api/http/mcp_servers.rs)
maps transport requests to the domain manager;
[`mcp_servers.rs`](../../../anyharness/crates/anyharness-contract/src/v1/mcp_servers.rs)
owns the wire types. Responses expose neither credential values nor command,
argument, or URL configuration. API errors use bounded codes and do not echo
configuration or filesystem details.

## Client surface

Settings → Agents → MCP servers is a thin editor over the current local
AnyHarness runtime. The pane supports stdio and HTTP creation, user defaults,
repository overrides and reset, and confirmed removal. Secret values are
entered once and never returned by list or mutation responses
([`McpServersPane.tsx`](../../../apps/packages/product-client/src/components/settings/panes/agents/mcp/McpServersPane.tsx)).
The SDK client is
[`McpServersClient`](../../../anyharness/sdk/src/client/mcp_servers.ts).

## Code map

```text
anyharness/crates/anyharness-contract/src/v1/mcp_servers.rs  public wire shapes and redacted Debug
anyharness/crates/anyharness-lib/src/persistence/sql/0078_user_mcp_servers.sql  durable catalog schema
anyharness/crates/anyharness-lib/src/domains/mcp_servers/
├── admission.rs  bounded user input validation
├── manager.rs    sealed credentials, catalog use cases, selection changes
├── model.rs      durable record and response projection
├── store.rs      SQLite CRUD and corruption handling
└── extension.rs launch selection and credential resolution
anyharness/crates/anyharness-lib/src/api/http/mcp_servers.rs  runtime HTTP boundary
anyharness/crates/anyharness-lib/src/domains/sessions/mcp_bindings/assembly.rs  product/user launch merge
anyharness/sdk/src/client/mcp_servers.ts  typed SDK methods
apps/packages/product-client/src/
├── lib/access/anyharness/mcp-servers.ts  runtime client adapter
├── hooks/access/agents/use-mcp-servers-access.ts  scoped query and mutation state
└── components/settings/panes/agents/mcp/McpServersPane.tsx  settings interaction
```

The SDK currently declares narrow local MCP wire types because the checked-in
generated OpenAPI artifacts do not yet include these routes and schemas. The
runtime OpenAPI source is authoritative; regenerate the SDK through
[`pnpm generate`](../../../anyharness/sdk/package.json) once the runtime
generator can execute.

## Observability and failures

The manager emits bounded `mcp_server.created`, `mcp_server.enablement_changed`,
and `mcp_server.removed` events. Launch emits `mcp_server.launch_prepared` with
the number of prepared servers. Events exclude secrets, command configuration,
URLs, and repository ids.

| Condition | Observable result | Recovery |
| --- | --- | --- |
| Secret-bearing create without the runtime data key | `503 MCP_SERVER_DATA_KEY_REQUIRED` | Configure `ANYHARNESS_DATA_KEY` and retry. |
| Invalid transport data, bounds, or selection | `400 MCP_SERVER_OPERATION_FAILED` | Correct the input; the response does not echo it. |
| Catalog or sealed-metadata corruption | `503 MCP_SERVERS_UNAVAILABLE` | Repair or remove the invalid durable record. |
| Selected sealed credentials cannot be opened during launch | A `not_applied` binding summary with the bounded missing-secret or resolver-error reason; the session launch continues without that server. | Restore the correct runtime data key or repair the record, then launch a new session. |
| Server removed or selection changed during a live session | No change to that session; later launches resolve current catalog state. | Start a new session to apply the change. |

## Proof

- [`manager.rs`](../../../anyharness/crates/anyharness-lib/src/domains/mcp_servers/manager.rs)
  tests sealing, key requirements, SQLite reopen, capacity, uniqueness, selection,
  redacted summaries, and corruption failure.
- [`admission.rs`](../../../anyharness/crates/anyharness-lib/src/domains/mcp_servers/admission.rs)
  tests bounds, HTTP schemes, control characters, and credential validation.
- [`extension.rs`](../../../anyharness/crates/anyharness-lib/src/domains/mcp_servers/extension.rs)
  tests launch credential resolution and `InternalOnly` exclusion;
  [`assembly.rs`](../../../anyharness/crates/anyharness-lib/src/domains/sessions/mcp_bindings/assembly.rs)
  tests merge order and summaries.
- [`mcp_servers_tests.rs`](../../../anyharness/crates/anyharness-lib/src/api/mcp_servers_tests.rs)
  tests authentication, all management routes, scope denial, validation,
  repository existence, and response redaction.
- SDK: [`mcp_servers.test.ts`](../../../anyharness/sdk/src/client/mcp_servers.test.ts).
- Settings: [`McpServersPane.test.tsx`](../../../apps/packages/product-client/src/components/settings/panes/agents/mcp/McpServersPane.test.tsx)
  and [`use-mcp-servers-access.test.tsx`](../../../apps/packages/product-client/src/hooks/access/agents/use-mcp-servers-access.test.tsx).
