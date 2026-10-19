# Local project retrieval

Status: runtime and composer controls authored; cloud qualification pending.

Project retrieval is an explicit, local source index scoped to one materialized
workspace. It uses SQLite FTS5 keyword ranking (BM25), not vector embeddings.
Indexing and searching require no model provider or account. Source content is
not sent to a model by these runtime operations.

## Surface and freshness

Runtime-owner local access:

- `GET /v1/workspaces/{workspace_id}/retrieval`: index state and bounded counts.
- `POST` on the same path: explicitly create or refresh the index.
- `DELETE` on the same path: clear cached source content and metadata.
- `POST …/retrieval/search` with `query`: refresh an existing index and return at
  most eight ranked excerpts, each with path, line range, content hash, and text.

Search never silently enables an index. Queries contain 1–32 terms and at most
512 UTF-8 bytes. Terms are quoted as FTS data, never interpreted as operator or
SQL syntax. Refresh hashes current files, replaces changed chunks, and removes
sources that were deleted, ignored, excluded, unreadable, or became non-text.
An unchanged hash skips database replacement. Failed refreshes do not return
stale search results. A fresh manager can reopen the persisted index.

## Composer integration

The local chat composer’s Project context picker reads status without enabling
an index. It remains available when agent setup blocks sending a prompt. Index,
refresh, and clear require explicit actions. Search results
show source and citations; the person selects excerpts before adding them to
the current draft. At most eight excerpts and 32 KiB of formatted context are
admitted. Source lines are quoted as reference data, not instructions. Sending
the prompt shares the selected text with the chosen provider. No model call
occurs during retrieval.

The picker remounts on runtime, actor, or workspace changes, suppresses telemetry
capture, and clears selections when the query changes. Busy operations block
additional writes. Queued-prompt editing does not expose this control.

## File and resource boundaries

Git lists tracked and untracked paths using standard ignore rules. A separate
ignored-tracked listing removes tracked files that now match those rules.
Enumeration has a 4 MiB output limit and 20-second deadline per Git command.
Ambient Git directory/index overrides are removed. Filesystem monitors and
untracked caches are disabled for discovery, and optional Git writes are off.
No repository scripts, filters, hooks, remote requests, or model calls run.

Eligible source extensions are explicitly listed in the file adapter. Common
credential paths, `.env` files, dependency/build directories, private-key files,
symlinks, binaries, invalid UTF-8, and files above 256 KiB are excluded. This is
an exclusion policy, not a guarantee that arbitrary source contains no secrets.
On Unix, descriptor-relative `openat` with `O_NOFOLLOW` anchors every source read
beneath the canonical workspace root. Non-regular files are rejected.

Refresh admits up to 10,000 eligible paths, 32 MiB of text, 20,000 chunks, and
20 seconds of source scanning. Counts and a truncation flag expose limits.
Chunks contain at most 64 lines and 8 KiB, with up to eight overlapping lines.
Oversized lines and files producing over 512 chunks are omitted. Empty chunks
are omitted. A truncated index is partial and must be presented as such.

## Storage, lifecycle, and authority

A private `project-retrieval/` runtime-home directory holds one SHA-256-named
SQLite cache per workspace. New files use mode 0600 on Unix. SQLite opens with
`SQLITE_OPEN_NOFOLLOW`; cache directory, file, and journal symlinks are refused.
The cache uses transactions, full synchronous writes, delete journals, and
schema version 1. It is disposable cached source, separate from the session log.

Index and search keep the workspace materialization read lease through Git and
the blocking worker, including when the HTTP caller disconnects. The workspace
row is resolved under that lease. Purge clears retrieval artifacts before the
workspace row dies; materialization cleanup also clears the cache before record
removal. This follows the existing exclusive lifecycle lease. Workspace claims
cannot index, search, or clear these owner-only local caches.

Operational events expose only index counts, truncation, and result counts.
Queries, source text, file paths, hashes, and workspace identities are excluded
from these records. API errors use bounded authored feedback.

## Code map and proof

- `adapters/git/project_sources.rs`: bounded Git enumeration.
- `adapters/files/retrieval.rs`: source eligibility and descriptor-relative reads.
- `domains/sessions/retrieval/`: chunking, query construction, cache and service.
- `api/http/project_retrieval.rs`: owner auth, workspace lease, and DTO mapping.
- `anyharness-contract/src/v1/retrieval.rs`: public transport schemas.

Coverage includes real Git ignores (including tracked ignored files), source
changes and deletion, restart persistence, workspace isolation, clear, private
file exclusions, bounds, citations, UTF-8, query syntax, symlinks, owner-only
routes, and record-removal cache cleanup. Shared operation-gate tests own the
underlying read/exclusive lock laws.

References: [Git enumeration](https://git-scm.com/docs/git-ls-files),
[SQLite open flags](https://docs.rs/rusqlite/0.40.0/rusqlite/struct.OpenFlags.html).

Owner: [Sessions](README.md).
