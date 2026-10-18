# Local project MCP servers

Current behavior. The [sessions spec](README.md) owns these launch inputs. The
macOS repository Environment pane manages external HTTP and stdio servers without
depending on organization integration definitions or an agent skill.

## Storage and management

`project_mcp_servers` holds one row per `(repo_root_id, name)`. Only the name,
transport, and enabled flag are plaintext. The entire server configuration is
encrypted with the runtime's `SessionDataCipher`; Desktop provisions its data key
from the keychain. A missing key refuses writes, and unreadable enabled settings
refuse launch with a redacted error. Disabling or removing a row does not need
the key. Repository deletion cascades to its rows. These settings are not part
of workspace mobility exports.

Runtime-owner management routes:

- `GET /v1/repo-roots/{repo_root_id}/mcp-servers` returns names, transports,
  enabled flags, and `encryptionAvailable`. It never returns configuration or
  credentials.
- `PUT …/mcp-servers/{name}` accepts `SessionMcpServer`, replaces all connection
  settings and credentials, and enables the server. The manager overrides caller
  connection IDs, server names, and catalog IDs with a project-managed identity.
- `PATCH …/mcp-servers/{name}` takes `{ "enabled": boolean }`.
- `DELETE …/mcp-servers/{name}` removes one row, idempotently.

Workspace-scoped JWTs cannot manage these repository-wide settings. The normal
runtime bearer-auth policy applies. Request errors omit submitted values and
JSON decoder details; change events record only the bounded action. There is no
new credential-reveal route.

Each repository accepts at most 32 servers, each serialized configuration at most
64 KiB. Names contain 1–50 ASCII letters, digits, underscores, or hyphens and
cannot start with `proliferate_`, case-insensitively. HTTP URLs accept HTTP or
HTTPS without userinfo or fragments. Header names must be unique ignoring case,
and header values must be valid single-line HTTP values. Stdio commands and
arguments are separate executable inputs, not shell scripts. Arguments are
bounded to 128 entries; headers and environment variables to 32 entries each.
Environment variable names are unique shell-style identifiers. Commands,
arguments, URLs, and credential values are individually bounded to 4096 bytes
and cannot contain NUL.

## Session launch

`ProjectMcpManager` implements `SessionExtension` and is registered in the app's
session-extension list. At each process launch it reads the workspace's
`repo_root_id`, decrypts enabled rows, validates them, and supplies concrete
servers to the existing launch assembler. The ACP
driver receives the HTTP headers or stdio command, arguments, and environment.
Internal-only session policy excludes project servers. Repository worktrees
share their repository's settings; another repository does not inherit them.

The local settings inventory describes saved launch configuration, not a
connection or tool-health result. Already running processes retain their current
MCP configuration. A management change
does not restart a process or revoke a server's external credentials.

## Desktop surface

The local repository Environment pane exposes **Project MCP servers** behind
`host.desktop`. The editor accepts one server JSON object with `transport` and
either `url`/`headers` or `command`/`args`/`env`. Header and environment objects
are converted to the runtime contract's name/value arrays. Replacing or removing
a server requires confirmation. Credentials remain in transient drafts until
save or navigation; successful save clears them. Neither user preferences nor
the mutation cache retain them. Switching runtime, actor scope, or repository
remounts the editor and discards its draft.

Inventory query keys include actor scope, runtime URL, and repository ID. UI
errors are authored messages; raw runtime response details are not displayed.
Missing key and unavailable-runtime states prevent saving.

Connectivity checks, tool inspection, OAuth setup, and per-agent selection are
not provided by this slice. Usable transports depend on the selected harness.

## Code and proof

- Domain and SQLite store: [`mcp_bindings/project/`](../../../anyharness/crates/anyharness-lib/src/domains/sessions/mcp_bindings/project/).
- HTTP: [`project_mcp.rs`](../../../anyharness/crates/anyharness-lib/src/api/http/project_mcp.rs).
- Contract: [`v1/mcp.rs`](../../../anyharness/crates/anyharness-contract/src/v1/mcp.rs).
- UI: [`ProjectMcpSettings.tsx`](../../../apps/packages/product-client/src/components/settings/panes/repo/ProjectMcpSettings.tsx), with workflow/access hooks and SDK `repoRoots` methods.
- Domain tests cover encryption, metadata-only inventory, project isolation,
  validation, missing keys, replacement, toggles, removal, and the server limit.
  Launch-assembly tests cover concrete ACP delivery, policy, disabled settings,
  and corrupt ciphertext. HTTP tests cover authenticated CRUD
  and sanitized errors. UI tests cover save, confirmation, cache/draft isolation,
  and unavailable states.
