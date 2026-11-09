# Skills

Status: current

Skills are runtime-local instruction packages imported from a folder containing
`SKILL.md` and optional resources. Settings → Agents → Skills manages the selected
runtime's packages. Importing preserves file bytes and starts the package disabled.
An imported package is immutable; import a new copy to update it.

## Selection

Each package has a user default and optional repository overrides. An explicit
repository boolean wins over the user default; resetting an override restores
inheritance. Repository selections use AnyHarness repo-root ids and therefore
apply to their worktrees too. These are the runtime user's settings, not
organization policy or repository-committed configuration.

Changing selections or removing a package affects subsequent agent launches.
Already running sessions retain the instructions and resources they received.
Skills cannot grant tools, MCP servers, permissions, credentials, or approvals.
Resource scripts run only when the agent invokes them under its existing policy.

## Import boundary

`POST /v1/skills` accepts named files with relative paths and base64 content.
The browser imports one selected directory; the server never follows a source
filesystem path or unpacks archive links. Admission requires a root `SKILL.md`
containing 1–65,536 bytes of UTF-8 instructions. Packages are bounded to 128 files
and 1 MiB total decoded bytes. Absolute paths, traversal, backslashes, colons,
control characters, empty components, case-insensitive duplicate paths, and file/directory collisions
are rejected. Binary resources are preserved. Catalog capacity is 64 packages.
Names contain at most 100 printable UTF-8 bytes.

`GET /v1/skills` returns metadata and selections, without resource contents.
`PUT /v1/skills/{id}/enabled` changes the user default or one existing repository's
override; `enabled: null` resets a repository override. `DELETE /v1/skills/{id}`
removes the package and all its selections. Routes use runtime authentication.

## Storage and launch delivery

`runtime_home/skills/packages.json` stores package bytes and selection state.
Writes use a private temporary file, sync, atomic rename, and parent-directory
sync. A runtime-local mutex serializes operations. Catalog corruption fails the
operation or launch rather than silently dropping selections.

The skills session extension resolves selections at agent startup and adds the
complete enabled `SKILL.md` text as a separate instruction block on the first
ACP prompt dispatched by each agent process, including resumed sessions. This
channel applies to every harness and does not depend on harness-specific session
metadata support or the session's prior turn history. Rejected or skipped turns
do not consume it. The harness receives the selected package directory alongside its
instructions so relative resource references have a base. Combined instructions
are bounded to 256 KiB per launch.

Resources are staged in a fresh
`<workspace>/.proliferate-skills/<launch-uuid>/<skill-id>/` directory, inside the
harness's workspace. Files are created exclusively. On Unix, descriptor-relative
`mkdirat`/`openat` with `O_NOFOLLOW` prevents checkout symlinks from redirecting
writes. The implementation does not modify existing repository files or automatic
tool policy. The generated staging directory is visible to repository tools; it
is not added automatically to Git ignore rules. Staged files remain after close
so session resume retains resource access; users may remove unused staging
folders after they no longer need those sessions. Harness file access still obeys
its configured permissions.

## Code map

- `anyharness-contract/src/v1/skills.rs`: public request and response shapes.
- `anyharness-lib/src/adapters/skills/`: bounded file admission and resource staging.
- `anyharness-lib/src/domains/skills/`: durable selection state, import and removal,
  and the session extension.
- `anyharness-lib/src/api/http/skills.rs`: runtime HTTP boundary.
- `anyharness/sdk/src/client/skills.ts`: typed client methods.
- `apps/packages/product-client/src/components/settings/panes/agents/skills/`:
  management UI; access and import workflows live in the corresponding hook
  directories.

## Observability

Bounded `skill.imported`, `skill.enablement_changed`, and `skill.launch_prepared`
records expose counts or operation outcomes. They exclude names, paths,
instructions, resource contents, and scope ids. API errors return a bounded
operation code without filesystem or package contents.

Owner spec: [README.md](README.md).
