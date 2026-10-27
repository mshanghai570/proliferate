# Delivery specification — daily-driver PR1: user MCP servers (frozen)

Chain position: PR 1 of the daily-driver campaign (1 user MCP servers → 2 extension activation
host → 3 codebase RAG). Evidence of record: the product-MCP pattern
([specs/systems/subagents/product-mcp-servers.md](../../specs/systems/subagents/product-mcp-servers.md)),
the runtime-local agent-packages precedent ([skills.md](../../specs/systems/harnesses/skills.md)),
and the complete-but-unwritten encrypted binding pipeline in
`anyharness/crates/anyharness-lib/src/domains/sessions/mcp_bindings/` (`encrypt_bindings` /
`decrypt_bindings`, `assemble_session_mcp_launch`, `acp::to_acp_servers`, binding summaries).
Builders implement from this document without re-deriving the architecture.

## Rulings (founder-approved 2026-10-26)

1. **Secrets at rest are sealed** with the existing `SessionDataCipher`; writes are refused
   without the data key, in plain words — never obfuscated silently.
2. **Stdio admission is free but bounded** — a user may attach any command they type; the
   bounds and redaction rules below apply, and the permission story is "the user attached it."
3. **Spec home** is a sibling of skills: `specs/systems/harnesses/mcp-servers.md`.

## Intent

A user adds their own MCP servers — a stdio command or an HTTP endpoint with secrets — in
Settings, enables them per repository the way skills enable, and every subsequent session on a
matching launch carries those tools. Today the launch pipeline consumes this exact data shape
and the driver already injects it into ACP `session/new`; nothing writes it and no surface
manages it. This PR closes the writer and the surface. Product outcome: "add the filesystem
MCP server in settings, ask any agent to touch files with it, it works — and never talks to
the network on my behalf otherwise."

## Acceptance gate (the merge bar — performed by Pablo, in the product)

In Settings → Agents → **MCP servers**: add a server (name, stdio command + args, one env
secret, one HTTP server with a header secret); it appears disabled. Enable it for the current
repo. Start a session; ask the agent to use the new tool — it performs the action, and the
session's MCP binding indicators list the server among product MCPs. A second repo with no
override launches **without** it while the first keeps it. Falsifiers: a secret or a decrypted
config in any log, summary, transcript, or wire response; a server still active in a session
launched after the override was removed; a code path that executed a server command outside a
harness launch.

## Scope

Spec landing (same PR — docs update with behavior per the repository law): a new sibling
[specs/systems/harnesses/mcp-servers.md](../../specs/systems/harnesses/mcp-servers.md) in the
skills.md grammar (runtime-local packages; user default + repository overrides; admission
bounds; launch delivery; code map; observability), the harnesses README index line, the
settings spec's pane row, and AGENTS.md unchanged (the harnesses router row already covers
this tree).

- **Schema**: one additive SQLite migration (`0078_user_mcp_servers.sql`): `id` (uuid), unique
  `server_name` (printable UTF-8, ≤100 bytes), transport (`stdio` | `http`), stdio fields
  (`command`, `args_json`, `env_sealed`), HTTP fields (`url`, `headers_sealed`), `description`,
  `enabled_user_default` (nullable bool), timestamps. `*_sealed` columns hold
  `SessionDataCipher` envelopes over the secret JSON (names plaintext, values ciphertext) —
  DB plaintext secrets are a found defect, not a gap to file later. Capacity 32 packages;
  catalog corruption fails charting operations rather than silently dropping selections.
- **Domain**: new `domains/mcp_servers/` — `model.rs` (record + selection state), `store.rs`
  (SQLite CRUD + selection precedence, reopen-safe), `admission.rs` (bounds: args ≤32, env
  count ≤16, header count ≤16, command/url bounded strings, control characters rejected,
  http/https schemes only, `connection_id` = `user:<server_name>`), `selection.rs`,
  `extension.rs` (`SessionExtension` that resolves user default + repo override for the
  launch's `repo_root_id`, honors `InternalOnly`, and emits `SessionMcpServer::Stdio/Http`
  values through `SessionLaunchExtras`; assembly and the driver need no changes).
- **API**: `api/http/mcp_servers.rs` — `GET /v1/mcp-servers` (metadata + selections, secrets
  never on the wire), `POST` (create), `DELETE /{id}`, `PUT /{id}/enabled`
  (`enabled: bool` sets user default; `enabled: {repo_root_id, enabled: bool|null}` writes or
  resets a repo override — skills' semantics verbatim). Contract in
  `anyharness-contract/src/v1/mcp_servers.rs`; responses omit paths that would leak home
  layout beyond what a secret's name reveals. Runtime bearer boundary applies.
  Workspace-scoped claims denied for global management, as extensions do.
- **Client**: SDK methods (`sdk/src/client/mcp_servers.ts` + generated types), Settings pane
  `components/settings/panes/agents/mcp/` beside the skills pane: server list with
  transport-identifying rows, add form (stdiocommand/args/env pairs; HTTP url/headers —
  secrets typed once, displayed as `•••`), enable/disable affordance, repo-override editor
  mirroring the skills section's control, confirm-before-delete. Copy owned in
  `copy/settings/agents/mcp/`; no fabricated empty states, runtime-disconnected shows the
  authored message.
- **Transcript visibility**: binding summaries from `summaries.rs` already surface the server
  name and ACP namespace — no new transcript event; MCP tool calls arrive as ordinary ACP
  tool calls (`mcp__<namespace>__<tool>` rendering already exists for product MCPs).

## Non-goals (deliberately out)

Marketplace distribution of MCP server definitions (later campaign PR) · org/team policy and
cloud/worker cross-runtime sync (the pipeline's cloud-compat section covers a later campaign) ·
per-session ad-hoc attach (the session-row `mcp_bindings_ciphertext` channel stays unwired) ·
OAuth discovery / MCP-native authentication flows (headers the user typed are v1's answer) ·
schema-driven tool capability editing; the harness's existing permission policy remains the
only gate on tool *use* · editing a record (delete + recreate; skills precedent).

## Proof

- `store_reopen` tests: reopen real SQLite, prove CRUD, unique name, capacity, corruption
  failure.
- Selection precedence tests: user default, repo override wins, reset restores inheritance,
  `InternalOnly` refuses all user servers, already-running sessions unaffected (skills'
  law verbatim).
- Assembly merge tests: user servers merge after product MCPs in launch order; summary list
  shows both; restart-required path still behaves when the data key is missing.
- Redaction tests: `Debug` output, logs, summaries, and all four endpoints prove secrets
  absent; `env_sealed` never decrypts outside the launch path.
- Endpoint router tests: auth, validation bounds, typed bounded error codes (no filesystem
  text), OpenAPI registration, workspace-scope denial.
- SDK tests + settings-pane vitest following the skills pane's suite shape (import/enable/
  override/delete, disabled-when-disconnected).
- `update_anyharness_schema_snapshot` regenerated; `python3 scripts/check_docs.py` green.

## Discharges

Build-list item "user-configurable MCP attach points" from the daily-driver ruling. The
skills fence ("Skills cannot grant MCP servers") is honored — this is a distinct system with
its own admission, not a skills extension.
