# Agent rules

Status: runtime and Settings editor authored; cloud qualification pending.

User-authored Markdown instructions select one agent kind, including a custom ACP
registration id. The runtime user default applies across repositories. An
explicit repository file replaces it and applies to that repository's worktrees.
An empty repository file suppresses inherited instructions. Removing it restores
the default. Each read resolves the current files; process launch and resume use
the same selection seam. Running processes retain their launch snapshot.

## API and storage

Runtime-owner authenticated `GET /v1/agent-rules/{agent_kind}` accepts optional
`repositoryId`. `PUT` accepts `repositoryId` and required `text`; `null` removes
the scope's file, while an empty string creates an explicit empty file. Unknown
repository ids are rejected. Workspace claims cannot access these global
settings. Errors contain authored codes and never instruction or path content.

Files are UTF-8, at most 64 KiB, and contain no NUL. Agent ids are at most 100
ASCII letters, digits, hyphens, underscores, or periods; `.` and `..` are refused.
Files are stored as `agent-rules/user/<agent-kind>.md` or under
`agent-rules/repositories/<SHA-256-of-repository-id>/<agent-kind>.md`. Scope
directories hold at most 128 files. New directories are private and files use
mode 0600 on Unix. Writes use an exclusive temporary file, sync, atomic rename,
and parent sync. Symlink sources and directory components are rejected. Reads
are bounded, require regular files, and use `O_NOFOLLOW` on Unix.

## Settings editor

Settings → Agents → Rules selects an actual catalog agent and optional repository.
The editor imports bounded UTF-8 Markdown/text files, saves exact text (including
an explicit empty override), and removes repository overrides with “Use user
default.” Dirty drafts lock the selectors until saved or discarded; refetches
preserve edits. Runtime or actor changes remount the editor and drop its draft.
Failed saves retain edits and offer retry. The form blocks telemetry capture.

## Delivery

The sessions extension adds a separate block explicitly identified as
user-authored rules to `launch_prompt_append`, on the first accepted prompt of
each agent process, including a resumed process. This shares the existing
harness-independent launch channel; it neither edits the durable authored prompt
nor rewrites `AGENTS.md`, `CLAUDE.md`, or other repository files. Rule text grants
no tools, credentials, MCP servers, or permissions and does not override product
policy. Skills remain a separate selection and delivery system.

## Code map and proof

- `anyharness-contract/src/v1/agent_rules.rs`: typed API shapes.
- `anyharness-lib/src/adapters/agent_rules/`: bounded Markdown IO.
- `anyharness-lib/src/domains/sessions/agent_rules.rs`: selection and launch block.
- `anyharness-lib/src/api/http/agent_rules.rs`: transport and repository validation.
- `anyharness-lib/src/api/agent_rules_tests.rs`: API auth and scope coverage.

Unit coverage includes persistence across manager restarts, per-agent and
repository isolation, exact Unicode/whitespace preservation, explicit empty and
inherited states, size and id validation, and symlink rejection. Launch-context
coverage verifies refreshed rules on resume and retained prior snapshots. API
coverage checks authentication, workspace-claim denial, DTOs and required text,
unknown repositories, and bounded errors. `agent_rules.saved` reports scope and
presence booleans without instructions, names, or paths.

Owner: [Sessions](README.md).
