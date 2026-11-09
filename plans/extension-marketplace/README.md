# Extension Marketplace — Plan & Status

A living plan for giving the Proliferate-based IDE extension discovery,
installation, management, and execution. This document is the project README for
the effort: it records what the codebase actually is today, what is feasible,
the architecture we intend to build, and how far along each stage is.

> **Status: partial implementation in the working tree.** Registry access,
> manifest/compatibility analysis, a verified local VSIX package installer, and
> an opt-in discovery/detail UI are present. Durable registration/removal is
> implemented and verified by runtime tests. Typed list/uninstall transport and
> runtime-backed Installed controls are connected. Contribution loading and
> an extension host remain absent. This is a discovery and management preview,
> not yet a functional extension system. Current behavior is owned by the
> [extension system spec](../../specs/systems/extensions/README.md).
> Update the [milestone tracker](#9-milestone-tracker) with each implementation
> change and record verification separately from code presence.

Last inspected: local working tree, 2026-10-12. The extension modules and this
README are currently untracked; “present” below does not mean committed or shipped.

**Architecture review gate:** approved by the user on 2026-10-12 (“you’re good
to continue”). Implementation proceeds incrementally on the existing IDE.
Category support will only be raised when a contribution or host capability
actually works; the detailed executable-host security decisions remain open.

---

## Table of contents

1. [Where we are](#1-where-we-are)
2. [Current IDE architecture (Phase 1 findings)](#2-current-ide-architecture-phase-1-findings)
3. [Existing extension support](#3-existing-extension-support)
4. [VS Code extension compatibility: feasibility](#4-vs-code-extension-compatibility-feasibility)
5. [Recommended extension-host architecture](#5-recommended-extension-host-architecture)
6. [Open VSX integration strategy](#6-open-vsx-integration-strategy)
7. [Files requiring modification or creation](#7-files-requiring-modification-or-creation)
8. [Compatibility limitations](#8-compatibility-limitations)
9. [Milestone tracker](#9-milestone-tracker)
10. [Open decisions](#10-open-decisions)
11. [Non-negotiable constraints](#11-non-negotiable-constraints)
12. [Authority and placement](#12-authority-and-placement)
13. [Current implementation slice](#13-current-implementation-slice--discovery-and-details)

---

## 1. Where we are

| Stage | Name | Status |
| --- | --- | --- |
| 1 | Inspect architecture and extension compatibility | **Done — this document** |
| 2 | Open VSX search and metadata retrieval | **Connected to discovery; target-aware downloads pending** |
| 3 | Marketplace UI | **Discovery/detail preview implemented; desktop proof pending** |
| 4 | Installation engine and management | **Verified local install/registration/removal; list/uninstall API and Installed controls connected** |
| 5 | Integrate supported extensions with the IDE | Not started |
| 6 | Compatibility detection and error reporting | **Manifest reporting present; runtime gating/crash reporting pending** |
| 7 | Updates, caching, performance | **Bounded caching, cancellation, and retries present; updates pending** |
| 8 | Full lifecycle testing | Backend package lifecycle verified; host lifecycle pending |

The architecture baseline from Stage 1 (before the new extension modules):

- Proliferate is an **AI IDE**: agent sessions over workspace worktrees, with a
  file browser, a syntax-highlighted **read-only** source viewer, terminals, and
  git review. It is not an editor kernel with plugins bolted on.
- There was **no extension host, plugin API, or marketplace** at the baseline.
  Discovery is now integrated, but no third-party extension execution exists.
- The closest existing analogues are **agent skills** and **MCP integrations** —
  neither is an IDE extension.
- Real filesystem editing is largely **delegated to external editors** through
  the Tauri shell (`open_in_editor`, `list_available_editors`).

Consequence: this is a **build-a-substrate** project, not a UI project. The
marketplace interface is the easy half; the extension host and install engine
are the hard half, and they define the milestones.

---

## 2. Current IDE architecture (Phase 1 findings)

This section records the architecture baseline before the extension additions.
The current extension implementation is tracked in section 9 and the owning spec.

### 2.1 The planes

Proliferate is a four-plane system ([`specs/ARCHITECTURE.md`](../../specs/ARCHITECTURE.md)):

```text
clients                    control plane              execution plane
Desktop / Web / Mobile  -> Server (FastAPI)  ->      cloud sandbox
  desktop only:            model gateway              supervisor
  local sidecar            (LiteLLM)                    worker + anyharness
```

- **Clients** (`apps/desktop`, `apps/web`, `apps/mobile`, shared
  `apps/packages/product-client`) render and interact. They are clients of the
  server's and runtime's contracts.
- **Server** (`server/`) is the hosted control plane: durable product state,
  policy, auth, integrations, billing. It never executes a session.
- **AnyHarness** (`anyharness/crates/anyharness*`) is the runtime that executes
  agent sessions over the ACP protocol, in a sandbox or as the desktop app's
  local sidecar. It owns workspace, file, terminal, and git truth over HTTP/SSE.
- **Model gateway** meters and routes agent LLM traffic.

The **desktop app is a deployment mode, not a fork**: the same runtime binary
runs locally as a sidecar, spawned by the Tauri shell
([`specs/systems/desktop-host/README.md`](../../specs/systems/desktop-host/README.md)).

### 2.2 Application framework

| Concern | Reality |
| --- | --- |
| Desktop shell | Tauri 2 (Rust) at `apps/desktop/src-tauri` |
| Product UI | React 19 + TypeScript, Vite, Tailwind 4 |
| Shared client | `@proliferate/product-client` — the connected Desktop/Web product |
| Design tokens | `@proliferate/design` |
| Client state | TanStack Query (remote truth) + Zustand stores (client-only) |
| Transport to runtime | `@anyharness/sdk` / `@anyharness/sdk-react` over HTTP/SSE |
| Transport to control plane | `@proliferate/cloud-sdk` / `-react` |
| Native bridge | typed `ProductHost` / `DesktopBridge`, `host.desktop !== null` |

Layout, layering, and dependency direction are enforced by
[`scripts/check_frontend_boundaries.py`](../../scripts/check_frontend_boundaries.py)
against [`specs/areas/frontend.md`](../../specs/areas/frontend.md).

### 2.3 Existing editor components

The "editor" is a **viewer plus buffer store**, not a mounted code editor:

| Piece | Path | What it is |
| --- | --- | --- |
| Source pane | `apps/packages/product-client/src/components/workspace/files/viewer/FileSourceView.tsx` | Virtualized, syntax-highlighted **read-only** source view (shiki tokens via `use-highlighted-lines` + `CodeTokenLine`) |
| Viewer frame/toolbar | `.../files/viewer/FileViewerFrame.tsx`, `FileViewerToolbar.tsx` | Tabs, toolbar, open-in-external |
| Diff pane | `.../files/viewer/FileDiffPane.tsx` | Review diff rendering |
| File tree | `.../files/tree/**` | Sidebar tree |
| Tabs/buffers state | `apps/packages/product-client/src/stores/editor/workspace-viewer-tabs-store.ts`, `workspace-file-buffers-store.ts`, `file-tree-store.ts`, `git-panel-ui-store.ts` | Open tabs, buffers, tree expansion, git panel |
| Write path | `apps/packages/product-client/src/hooks/workspaces/workflows/files/use-workspace-file-buffer-actions.ts` | Conflict-aware buffer writes (runtime is the file authority) |
| Scratch pad editor | `apps/packages/product-client/src/components/workspace/scratch/ScratchCodeMirrorEditor.tsx` (+ hooks under `hooks/workspaces/lifecycle/scratch-codemirror-*`) | A **CodeMirror 6** scratch editor, not a workspace file editor |
| Chat composer | `.../chat/input/ComposerRichTextEditor.tsx` | Lexical rich-text input |
| Terminal | `.../terminals/**` + `hooks/terminals/lifecycle/use-xterm-surface.ts` | **xterm.js** PTY surface |

**Important finding:** `@monaco-editor/react` is declared as a dependency in
`apps/packages/product-client/package.json` and `apps/desktop/package.json`, and
a theme exists at `apps/packages/product-client/src/lib/infra/editor/monaco-theme.ts`,
but **no source file mounts Monaco** and the theme module has no importers. It is
dormant scaffolding, not a working editor integration. Any plan that assumes
"we already have Monaco" is wrong today.

### 2.4 Extension, plugin, and command infrastructure

- **No extension host.** No `vscode` API shim, no extension `package.json`
  activation logic, or integrated extension marketplace. The new VSIX adapter
  inspects/extracts archives but is not an installation service.
- **No plugin API for the IDE.**
- **Command registration:** app-level commands exist as data
  (`apps/packages/product-client/src/config/**`, command palette under
  `components/workspace/shell/command-palette/**`), but there is no third-party
  command contribution surface.
- **Settings:** client-side preferences persist per user through Zustand stores
  bridged by lifecycle hooks (e.g. `stores/preferences/workspace-ui-store.ts`,
  `lib/domain/preferences/appearance.ts`). Server owns durable account settings
  (`specs/systems/settings/README.md`). There is no extension-contributed
  settings schema.
- **Language servers:** none. No LSP client, protocol types, or server
  lifecycle code exists in the clients or in AnyHarness.
- **Terminal:** real PTYs owned by AnyHarness
  (`anyharness-lib/src/domains/terminals/**`, `live/terminals/**`), rendered by
  xterm in the client.
- **The one real runtime extension point:** the Rust trait
  `SessionExtension` in
  `anyharness/crates/anyharness-lib/src/domains/sessions/extensions.rs`,
  implemented by cowork, reviews, and delegated-agent delivery and wired by
  `app/`. It is a **compile-time, in-process, first-party** seam — not a
  user-installable plugin API.
- **Package management in the repo:** pnpm workspace + Cargo workspace for the
  product itself. There is no user-facing package manager, download service, or
  integrity/verification pipeline for third-party artifacts.

### 2.5 Lifecycle and security boundaries

- **Desktop shell** owns local process lifecycle: spawning and supervising the
  AnyHarness sidecar and the desktop worker, local secrets under 0600 files, and
  the at-rest data key in the Keychain. Sidecar env is assembled from local
  secrets, agent seed env, login-shell `PATH`, and the control-plane origin.
- **Product runtime** owns workspaces, worktrees, files, terminals, and git in
  local SQLite (`anyharness-lib/src/persistence/**`, with migrations).
- **Agent LLM traffic** goes through the gateway; **third-party system access**
  goes through the integration gateway so credentials never enter an
  environment.
- Product code checks `host.desktop`, never a surface string; Desktop-only
  capabilities live behind `DesktopBridge`.

---

## 3. Existing extension support

The honest answer: **none for IDE extensions.** Several features look
extension-shaped and are worth reusing, but none is an installable plugin.

| Existing concept | What it actually is | Why it is not an IDE extension |
| --- | --- | --- |
| **Agent skills** (`specs/systems/harnesses/skills.md`, skills tool-result UI) | Markdown instruction bundles injected into an agent's system prompt | Read by the *agent*, not by the IDE; contributes no UI, commands, or editor behavior |
| **MCP integrations** (`specs/systems/integration_gateway/README.md`, `domains/sessions/mcp_bindings/**`) | Tool/context servers bound to a session | Agent-facing capability, resolved per call; not an IDE extension |
| **Agent catalogs** (`catalogs/agents`, `domains/agents/**`) | Harness/model distribution and readiness metadata | First-party, versioned by the repo, not user-installed |
| **`SessionExtension` trait** | Compile-time session lifecycle seam | Requires a rebuild; not third-party-installable |
| **Tauri commands** | Shell affordances (`open_in_editor`, `list_available_editors`, …) | Native bridge, not a contribution surface |

**Verdict:** there is no substrate to extend. Discovery, install, host, and
management all have to be built, and the design must decide *what* a
"Proliferate extension" is before it decides how to install one.

---

## 4. VS Code extension compatibility: feasibility

### 4.1 What a VS Code extension actually needs to run

A `.vsix` is a zip with a manifest (`package.json` `contributes` + `main`) and
code. Executing one requires, at minimum:

1. A **Node (or web) extension host process** that loads `main`, enforces
   activation events, and disposes on deactivate.
2. The **`vscode` module surface** — `commands`, `window`, `workspace`,
   `languages`, `debug`, `tasks`, `extensions`, `env`, `ThemeColor`, event
   emitters, `Uri`, `Disposable`, `CancellationToken`, `EventEmitter`.
3. Editor/workspace **capability providers** behind that surface: document
   models with edit application and versioning, workspace folders, virtual file
   systems, diagnostics collections, completion/hover/definition/symbol
   providers, code actions, formatting edits, semantic tokens, tree views,
   webview panels, task and debug adapters, terminals, and configuration.
4. **Language protocol plumbing** for the popular subset: LSP client (and DAP
   for debuggers), plus the ability to *launch and supervise* language server
   processes.
5. **Contribution activation**: themes, grammars (TextMate `grammars` +
   `languages`), snippets, and keybindings are declarative and can be honored
   without executing JS at all.

### 4.2 What Proliferate has today

| Requirement | State |
| --- | --- |
| Node extension host | Absent |
| `vscode` API shim | Absent |
| Document/edit model with versioning | Partial — read-only viewer + conflict-aware file buffers; no edit application surface |
| Theme contribution | Effectively yes (client theming + `monaco-theme.ts` exists dormant) |
| Grammar/tokenizer contribution | Read-only viewer uses **shiki** (TextMate grammars); no way to register a contributed grammar |
| Snippet contribution | Absent |
| Diagnostics / completion / hover providers | Absent |
| LSP client + server supervision | Absent |
| DAP / debug adapter | Absent |
| Webviews | Absent (a webview container can be built; there is no panel host) |
| Terminal API | Present in the runtime; no extension-facing surface |
| File system API | Runtime has file APIs; no extension-facing surface |
| Configuration API | Client+server settings exist; no extension config schema |
| Isolated execution | Sidecar/worker give a process boundary to reuse; no per-extension isolation |

### 4.3 Feasibility, tiered

Do **not** claim universal VS Code compatibility. The realistic tiers:

| Tier | Categories | Feasibility | Depends on |
| --- | --- | --- | --- |
| **A — declarative, no code** | Color themes, icon themes, maybe file-icon themes | **High** | A contribution loader that maps manifest `contributes` into existing token/theme plumbing |
| **B — declarative grammars/snippets** | Language grammars, snippets, keybindings (for our own command set) | **Medium–High** | Registering TextMate grammars with the highlighter, a snippet engine, keybinding dispatch |
| **C — host-backed, well-specified** | Language servers, formatters, linters, some productivity extensions | **Medium** | A Node extension host + `vscode` shim covering documents, diagnostics, providers, config, and LSP process supervision |
| **D — deeply editor-coupled** | Debuggers (DAP), webview-heavy panels, tasks, VCS UIs, notebook-ish surfaces | **Low–Medium**, per extension | DAP client, webview host, task API, terminal/task surfaces |
| **E — full API consumers / native modules** | Extensions that bundle node-gyp/native addons, spawn privileged processes, or use broad `vscode` API | **Low** | Process isolation + a large shim; some will simply be unsupported |

Each unsupported extension must surface **why**, in the marketplace UI, rather
than failing at activation.

---

## 5. Recommended extension-host architecture

### 5.1 The core decision

Two viable directions, and the recommendation is to sequence them rather than
choose one:

- **Track 1 — native extensions (first-party + trusted).** A small,
  product-native contribution surface contributed by the repo and vetted
  partners: manifest + declarative contributions + a controlled runtime hook.
  Cheap, safe, immediately useful.
- **Track 2 — VS Code compatibility host (third-party).** An isolated Node
  process running a `vscode` API shim, fed by capability providers, enabling
  Tiers A–C above and honest Tier D per-extension.

Track 1 makes the marketplace useful early. Track 2 is what makes it *feel* like
the ecosystem the user asked for, and it is the expensive part. Building Track 2
first would stall the project behind a shim with no UI to justify it.

### 5.2 Layered design

```text
UI            marketplace surfaces (product-client)
                 components/extensions/**            cards, detail, installed list
                 hooks/access/extensions/**          query/mutation wrappers
                 hooks/extensions/{derived,workflows,lifecycle}/**
              |
Registry      ExtensionRegistry abstraction (product-client lib/access)
              + Open VSX adapter
              + (later) private/local adapters
              |
Control       Extension service (AnyHarness domain) — the durable authority
              install/uninstall/enable/disable/update/state
              |
Engine        Install engine (AnyHarness adapters)
                 download -> verify -> unpack safely -> validate manifest
                 -> resolve compatibility -> register -> rollback on failure
              |
Host          Extension host process (isolated)
                 Track 1: native contribution loading
                 Track 2: Node session running a vscode API shim
              Capability providers bridge host <-> runtime (files, workspace,
              commands, diagnostics, terminals, config, webviews)
```

Design rules that fall out of the existing architecture:

- **The runtime is the authority.** Installed extensions, their state, and their
  enablement are durable runtime truth, owned by an AnyHarness
  `domains/extensions/**` domain and its SQLite tables — exactly like
  workspaces, sessions, and terminals. The client never invents existence.
- **The client renders and calls.** Marketplace UI is `product-client`
  components over access hooks; registry/management calls cross a typed seam.
  No marketplace filesystem or archive logic in the UI layer.
- **Native capability crosses narrowly.** If the host needs a fact or an effect
  it does not own (open a terminal, read a file, apply an edit), it crosses as a
  narrow capability trait/handler wired in `app/` — never by reaching into
  stores or spawning its own privileged process.
- **Isolation is a process boundary.** Follow the existing supervisor/worker
  precedent: the host is a supervised child with a bounded protocol, restarted
  on crash, never in-process with the runtime.
- **Mirror the runtime contract split** (`domains/` durable vs `live/` running):
  extension *installation* is durable; extension *host processes* are live.

### 5.3 Registry abstraction

Adapt the requested interface to each language, keeping it registry-agnostic.
TypeScript (client-side, over the runtime or directly over the registry):

```ts
interface ExtensionRegistry {
  search(query: ExtensionSearchQuery): Promise<ExtensionSearchResult>;
  getExtension(namespace: string, name: string): Promise<ExtensionMetadata>;
  getVersions(namespace: string, name: string): Promise<ExtensionVersion[]>;
  download(extension: ExtensionMetadata, version: string): Promise<ExtensionPackage>;
}
```

Rust (runtime-side, where download + install execute):

```rust
// anyharness-lib/src/domains/extensions/registry.rs
#[async_trait]
pub trait ExtensionRegistry: Send + Sync {
    async fn search(&self, query: ExtensionSearchQuery) -> Result<ExtensionSearchResult, RegistryError>;
    async fn metadata(&self, namespace: &str, name: &str) -> Result<ExtensionMetadata, RegistryError>;
    async fn versions(&self, namespace: &str, name: &str) -> Result<Vec<ExtensionVersion>, RegistryError>;
    async fn download(&self, id: &ExtensionId, version: &str) -> Result<ExtensionPackage, RegistryError>;
}
```

Open VSX implements it first; private registries, local `.vsix`, and a local
development folder are later adapters behind the same trait.

### 5.4 Security boundaries (Phase 7 requirements, designed in from the start)

| Risk | Mitigation |
| --- | --- |
| Archive path traversal (`../`, absolute paths, symlinks) | Reject during unpack (Track 2); never extract outside the target dir |
| Malicious/tampered package | Verify registry-provided integrity/signature metadata where available; record hash; reject on mismatch |
| Extension code reaching secrets/credentials | Host never sees Keychain items, auth session files, or gateway credentials; capability bridge grants only explicit, narrow powers |
| Privileged command execution | Extensions must not receive Tauri or runtime credentials; a bridge must authenticate and authorize declared capabilities |
| Crash takes down the IDE | Host runs in a supervised child process with bounded protocol; restart, disable, and surface the error |
| Untrusted publishers | Publisher/namespace verification surfaced in UI; explicit warning before first activation of code-bearing extensions |
| Blocking the UI | Registry fetches are async, cached, rate-limit-aware; startup loads no extension code until activation events fire |

---

## 6. Open VSX integration strategy

### 6.1 Why Open VSX

Open VSX (Eclipse) is the open, vendor-neutral registry for VS Code-format
extensions, used by VS Code-compatible editors. It publishes documented REST
APIs, so it can be consumed without scraping, and without touching Microsoft's
proprietary Visual Studio Marketplace endpoints or redistribution rights.

### 6.2 Endpoints (verify against `/swagger-ui` when implementing)

Server: `https://open-vsx.org`

| Purpose | Endpoint |
| --- | --- |
| Search | `GET /api/-/search` — `query`, `category`, `size`, `offset`, `sortBy`, `sortOrder`, `includeAllVersions` |
| Query by id | `GET /api/-/query` — extension id / namespace / name / version |
| Latest metadata | `GET /api/{namespace}/{name}` and `GET /api/{namespace}/{name}/latest` |
| Version metadata | `GET /api/{namespace}/{name}/{version}` |
| File access | `GET /api/{namespace}/{name}/{version}/file/{path}` |

The metadata response carries a `files` block (`download`, `icon`, `readme`,
`license`, `changelog`, …). **Prefer the returned `files.download` URL** over
constructing a path, and show readme/changelog/screenshots from the returned
file URLs.

### 6.3 Behaviors to implement

- **Search**: name/description/publisher/category query; category filters;
  sorting by relevance/popularity/rating/update date; pagination via
  `size`/`offset`.
- **Metadata + versions**: latest and per-version metadata, version history,
  dependencies, capabilities, publisher.
- **Compatibility targeting**: resolve the registry's target-platform variants
  (universal vs platform-specific, e.g. darwin-arm64) and pick the best match
  for the host; record the choice.
- **Caching**: cache search and metadata in the query cache; never re-fetch
  unchanged metadata. Conditional `ETag` requests are not usable cross-origin
  because the registry's CORS preflight does not allow `If-None-Match`.
- **Rate limits**: honor `X-RateLimit-*` headers and `429` `Retry-After`;
  back off with jitter; surface a non-alarming state in the UI.
- **Updates**: background check of installed extensions against registry
  metadata; never block the editor.
- **Politeness**: no crawling; no scraping of webpages. A browser/WebView
  cannot set `User-Agent`, and custom headers would trigger the denied
  preflight, so requests carry only the safelisted `Accept` header.

### 6.4 Current adapter behavior and remaining integration

- **Search entries and metadata documents use different wire models.**
  `/api/-/search` entries carry no `targetPlatform` and no `preRelease`; only
  the `/api/{namespace}/{name}` metadata document does. Compatibility decisions
  must therefore be made from metadata, never from a search result.
- **`allVersions` includes a `latest` alias** that is not a real version and
  must be dropped when rendering a version list.
- **The registry sends `ETag` with `cache-control: no-cache`**, but its CORS
  preflight does not allow `If-None-Match`, so a WebView cannot use conditional
  requests against it. TanStack Query owns response caching above the transport.
- **`Retry-After` may be seconds or an HTTP date**; both forms are parsed.

- The transport adapter is stateless; TanStack Query holds search and metadata
  responses. Requests accept query cancellation; cache persistence and
  background extension update checks remain absent.
- Retries are bounded by attempt count and honor `Retry-After` up to 30
  seconds per attempt. Longer delays return a rate-limit error for later retry;
  jitter is not implemented. Configurable registry URLs and a default
  public origin exist; private-registry authentication does not.
- Platform selection is a pure helper only. Metadata and package resolution
  do not yet request a selected platform variant, so target-aware download
  resolution remains open.

### 6.5 Credentials and configuration

Registry base URL and optional auth are configuration, never hardcoded. A
private registry later reuses the same abstraction with its own auth.

---

## 7. Files requiring modification or creation

Paths are repo-relative. `(new)` marks proposed files; existing foundations
are identified explicitly below.

### Clients (`apps/packages/product-client`)

| Path | Change |
| --- | --- |
| `src/components/workspace/shell/sidebar/SidebarPrimaryNavigation.tsx` + `ProductSidebarNavigation.tsx` | Add the Extensions entry to primary navigation (the "activity bar") |
| `src/components/extensions/**` (new) | Marketplace list, cards, detail view, installed/publisher/error surfaces |
| `src/pages/**` or workspace right-panel host | Route/pane for the Extensions surface |
| `src/hooks/access/extensions/**` (new) | Query/mutation wrappers, query keys, invalidation, retry |
| `src/hooks/extensions/{derived,workflows,lifecycle}/**` (new) | Installed-list view model, install/update workflows, background update checks |
| `src/lib/access/extensions/open-vsx-registry.ts` (present) | Registry adapter; connect to marketplace access hooks |
| `src/lib/access/anyharness/extensions.ts` (new) | Runtime extension-management transport |
| `src/lib/domain/extensions/**` (present) | Registry models/mapping and manifest/compatibility rules; integrate with runtime policy and UI |
| `src/stores/extensions/**` (new) | Client-only marketplace UI state (filters, selection, query) |
| `src/primitives/**` | Reuse existing primitives only; no new DOM primitive duplicates |

### Runtime (`anyharness`)

| Path | Change |
| --- | --- |
| `anyharness/crates/anyharness-lib/src/domains/extensions/**` (new) | Durable domain: model, store, service, runtime, registry trait, install policy |
| `anyharness/crates/anyharness-lib/src/adapters/extensions/**` (partially present) | VSIX inspection/extraction exists; download, integrity verification, staging, rollback remain |
| `anyharness/crates/anyharness-lib/src/live/extensions/**` (new) | Supervised extension-host process lifecycle |
| `anyharness/crates/anyharness-lib/src/domains/sessions/extensions.rs` | Register existing session extension point where relevant |
| `anyharness/crates/anyharness-lib/src/api/http/extensions*.rs` (new) | HTTP handlers + error mapping |
| `anyharness/crates/anyharness-contract/src/v1/**` | Wire schemas for search/metadata/install/state |
| `anyharness/crates/anyharness-lib/src/app/**` | Dependency wiring (`AppState`) |
| `anyharness/crates/anyharness-lib/src/persistence/**` | Migrations for extension tables |
| `anyharness/sdk/**`, `anyharness/sdk-react/**` | Regenerated contract consumers |

### Desktop host (`apps/desktop/src-tauri`)

| Path | Change |
| --- | --- |
| `src/lib.rs` | Register any new shell command (only if a capability requires it) |
| `src/**` | Possibly a host-process supervisor if the extension host must be a shell-owned child rather than a runtime-owned one |

### Server (`server/`)

Only if extension state must be **account-synced** (extension sync is explicitly
out of scope for the core; see [Open decisions](#10-open-decisions)).

### Documentation

| Path | Change |
| --- | --- |
| `specs/systems/<new-system>/README.md` (new) | The owning system spec, once design is approved |
| `specs/README.md` | One new line in the system index |
| `specs/areas/frontend.md`, `specs/areas/anyharness.md` | Note new source areas in the code maps |
| `guides/**` (only if a procedure is needed) | A task guide, referenced from the owning spec |

---

## 8. Compatibility limitations

State these plainly in the product, not just here:

1. **Installation ≠ execution.** A `.vsix` can install successfully and still
   not run, because the host may not implement the APIs it needs. The UI must
   show "installed, unsupported APIs: …" rather than implying success.
2. **No universal VS Code compatibility claim.** Only demonstrated categories
   are supported; everything else is labeled.
3. **No proprietary marketplace.** Open VSX only; Microsoft Marketplace
   endpoints are not used without authorization.
4. **Read-only viewer today.** Extensions that assume an editable document model
   cannot function until the editor surface (or an edit-application path through
   buffers) exists. This is the single biggest capability gap.
5. **No LSP or DAP yet.** Language servers, formatters, linters, and debuggers
   are Tier C/D and require new client + host plumbing.
6. **No webview host yet.** Webview-heavy extensions are unsupported until a
   panel host and a message bridge exist.
7. **Native addons and broad API consumers** will remain unsupported; some are
   categorically unsafe to load.
8. **Per-workspace enablement** requires a workspace→extension association the
   runtime must own; it is not free.
9. **Extension settings** need a schema/contribution model that does not exist
   yet; until then, extensions that require configuration cannot be fully
   configured from the UI.

---

## 9. Milestone tracker

Each stage names a deliverable and an exit criterion. Code presence is recorded
from source inspection. Tests in the tree establish intended coverage, not a
passing result. Earlier test-count claims had no command/output evidence in this
README and were replaced with coverage inventories. Fresh verification from
this implementation pass is recorded at the end of this document.

### Stage 1 — Inspect architecture and extension compatibility · **DONE**

- Deliverable: this document.
- Exit criterion met: architecture, existing support, feasibility, recommended
  host, Open VSX strategy, file list, and limitations are recorded from source
  inspection (not from memory).

### Stage 2 — Open VSX search and metadata retrieval · Adapter present

Present under `apps/packages/product-client/src/`:

| File | Role |
| --- | --- |
| `lib/domain/extensions/extension-models.ts` | Registry vocabulary, the `ExtensionRegistry` port, `extensionId` / `nextPageOffset` |
| `lib/domain/extensions/extension-registry-error.ts` | Typed failure codes (`not-found`, `rate-limited`, `http-error`, `malformed-response`, `network-error`) |
| `lib/domain/extensions/open-vsx-wire.ts` | Raw registry JSON shapes (permissive, wire-only) |
| `lib/domain/extensions/open-vsx-mapping.ts` | Pure wire→domain mapping; the one place that decides what a response means |
| `lib/access/extensions/open-vsx-registry.ts` | The adapter: URL building, CORS-safe requests, 429 backoff, JSON + error mapping |

- Test files present: `open-vsx-mapping.test.ts` and
  `open-vsx-registry.test.ts`. They cover mapping, search/pagination, CORS-safe
  request headers, retry behavior, typed failures, and package URL resolution.
  Passing results are not established by this source inspection.
- Stage 3 now connects the adapter through scoped React Query access hooks
  and the opt-in discovery surface. No VSIX bytes are downloaded by the client.
- **Platform targeting remains partially implemented:** compatibility helpers
  exist, but selecting a platform-specific metadata/download endpoint is not
  wired into the adapter. `download` returns package URLs, not VSIX bytes.

### Stage 3 — Marketplace UI · Discovery/detail preview implemented

- Deliverable: Extensions entry in primary navigation; discovery (search,
  featured/popular/recent/recommended, category filter, sorting); extension
  cards; detail view (readme, screenshots, versions, dependencies, capabilities,
  publisher, requirements, changelog, repo link, controls).
- Present: lazy `/extensions` route and sidebar Blocks icon, debounced public
  search, category/sort controls, pagination, extension cards, metadata,
  README/changelog with safe Markdown/images, version history, dependencies,
  publisher, requirements, and manifest capability reporting. Existing design
  tokens and Button/Input/Select/Badge primitives are reused.
- `VITE_EXTENSION_MARKETPLACE=1` enables the preview; it is off by default.
  Installation is explicitly disabled, and Installed reports that its manager
  is unavailable. No installed records or recommendations are fabricated.
- Still open: featured/recommended sources, actual installed-state controls,
  and full desktop visual/performance proof. No connected browser was available
  during this pass. A live Open VSX API search succeeded; hermetic surface tests
  exercise the adapter/UI seam with deterministic responses.
- Exit criterion remains: rendered UI in the running desktop app using the
  project's design language, loading real registry data without blocking the
  editor. Code/test presence alone does not clear that criterion.

### Stage 4 — Installation engine and management · Partial (verified installation and durable management)

- Deliverable: durable install domain + adapters: download, verify, safe unpack
  (traversal/symlink/bomb guards), manifest inspection, compatibility check,
  isolated install directory, registration, enable/disable, uninstall, update
  detection, state restore across restart, local `.vsix` install, rollback.
- Present: `domains/extensions/installer.rs` provides `install_local_vsix` for
  local packages with a caller-supplied SHA-256 and exact requested identity.
  `adapters/extensions/package.rs` snapshots and verifies the source before
  inspection/extraction, stages privately in the runtime home, writes a receipt,
  and publishes an immutable version directory under an exclusive file lock.
- The installer rejects declared Node/browser entrypoints, unresolved
  dependencies/extension packs, unsupported contribution types, mismatched
  publisher/name/version, invalid identities, and missing declared resources.
  It accepts declarative package materialization only; installation does not
  establish API compatibility or enable an extension.
- Archive preflight rejects traversal, symlinks, Windows drive/stream/device
  aliases, and normalized/case-folded path collisions before output. Extraction
  requires an empty destination, reads manifests within 1 MiB, bounds declared
  and actual expansion, and creates files exclusively with private Unix modes.
- Staging rejection removes its own files. Publication preserves existing
  versions, refuses overwrites, and rolls back a failed parent-directory sync.
  A process crash may leave private staging residue; there is no startup sweep
  or OS sandbox against another process running as the same user.
- Package snapshots are limited to 128 MiB compressed; extraction permits at
  most 4,096 entries, 32 MiB per entry, and 256 MiB total. Windows inherits the
  runtime home's ACLs; directory fsync is available on Unix only.
- Executed isolated source tests: 36 passed. Coverage includes hostile archives,
  checksum/CRC failures, snapshot independence, permissions, identity/resource
  admission, receipt spoofing, concurrent publication, and rollback. A real
  `vscode.theme-defaults@1.95.3` Open VSX VSIX also materialized successfully
  against its registry-provided SHA-256 with a persisted receipt. This proves
  package installation, not visible theme application.
- Still open: native download plumbing, publisher-signature verification,
  supported dependency resolution, workspace enablement, runtime
  HTTP/SDK transport, Installed UI controls, updates, and activation.
- Exit criterion: the full lifecycle runs against real packages and state
  survives a runtime restart, proven by executed tests (including a hostile
  archive test).

### Stage 5 — Integrate supported extensions with the IDE · Not started

- Deliverable: contribution loader for Tier A/B; extension host for Tier C
  capabilities that are actually implemented; honest runtime gating.
- Exit criterion: at least one theme and one grammar/snippet extension visibly
  change the IDE; a Tier C extension's capability works end-to-end *or* is
  labeled unsupported with a named missing API.

### Stage 6 — Compatibility detection and error reporting · Analysis modules present

Present (pure domain, no I/O):

| File | Role |
| --- | --- |
| `lib/domain/extensions/extension-manifest.ts` | The manifest subset we reason about: identity, `engines.vscode`, entrypoints, activation events, dependencies, and `contributes` mapped onto a closed capability vocabulary |
| `lib/domain/extensions/extension-compatibility.ts` | Host platform targeting, three-valued engine-range interpretation, capability support, and the ordered compatibility verdict |

The rules are deliberately **three-valued** where the answer is not knowable:
`interpretEngineRequirement` returns `satisfied` / `unsatisfied` / `indeterminate`,
and only `satisfied` clears a blocker. A range we cannot model (`1.2.3 - 2.0.0`,
`||` unions, unknown syntax) is reported `indeterminate` rather than guessed,
because a confident wrong answer is the failure mode this project must avoid.

Version *ordering* is not re-implemented: it reuses `compareSemver` from
`lib/domain/capabilities/version-compat.ts`, which already owned that job.

- `extension-manifest.test.ts` and `extension-compatibility.test.ts` contain
  coverage for manifest mapping, platform helpers, engine ranges, and blocker
  ordering. They were not run during this documentation update.
- Compatibility is a manifest-based assessment, not execution proof. Required
  follow-ups include complete caret handling for `0.0.x`, unknown host-platform
  handling, and rejecting executable entrypoints independently of contribution
  declarations. The analyzer does not inspect JavaScript API usage.
- **Honest baseline:** `EMPTY_EXTENSION_HOST_CAPABILITIES` declares engine
  `0.0.0` and no supported capability, because no extension host exists yet.
  This is a conservative starting point, but the analyzer is not wired into
  install or activation policy and must not be treated as an execution gate.
  That constant must be raised only
  alongside host work that makes a capability real.
- Present: manifest capabilities, API requirements, and the missing-host
  explanation in extension details. Still open: runtime policy enforcement,
  contained host-crash reporting, and installed-but-unsupported state.

### Stage 7 — Updates, caching, performance · Partial (registry cache/retries)

- Present: CORS-safe request headers, 429 retries with bounded waiting, query
  cancellation, lazy route loading, and lazy selected-extension documents.
- Pending: background update checks, update-all, persistent cache policy, lazy
  contribution loading, and startup performance measurement.
- Exit criterion: measured — startup is unaffected by installed extensions and
  marketplace interaction does not block the UI (timings recorded).

### Stage 8 — Full lifecycle testing · Backend package lifecycle verified; activation pending

- Deliverable: lifecycle test suite (install→enable→update→disable→uninstall→
  restart), security tests (traversal, tamper, crash), and registry contract
  tests against a stub server.
- Exit criterion: the suite runs green in CI, executed and reported.

---

## 10. Open decisions

These need an owner decision before the affected stage starts. Recommendations
are proposals; silence does not satisfy the architecture approval gate.

| # | Decision | Options | Recommendation |
| --- | --- | --- | --- |
| D1 | What is a "Proliferate extension"? | (a) VS Code-compatible only; (b) native-only; (c) both tracks | **(c) both, sequenced** — native first for early value, VS Code host next |
| D2 | Where does the extension host run? | (a) runtime-owned supervised child; (b) Tauri-shell-owned child; (c) in-renderer | **(a)** — matches supervisor/worker precedent and works for cloud too |
| D3 | Extension state location | (a) runtime SQLite only; (b) runtime + account sync on the server | **(a)** for the core; extension sync is a later, separate system |
| D4 | Editable document model | (a) build native edit surface; (b) adopt a code editor kernel; (c) stay read-only for now | Decide with Stage 5; (**b**) is the cheapest path to Tier C |
| D5 | Sandbox for untrusted code | (a) plain child process + capability bridge; (b) OS sandbox (seatbelt/landlock) | A child process isolates crashes, but cannot by itself protect filesystem credentials. Resolve OS isolation and capability enforcement before enabling untrusted code |
| D6 | Ship behind a flag? | (a) always-on; (b) feature flag | **(b)** until Stage 5 proves non-disruption |

---

## 11. Non-negotiable constraints

Carried from the project brief and the repository rules; treat as acceptance
criteria, not aspirations.

- Preserve all existing Proliferate functionality; changes are incremental and
  reversible.
- Do not rewrite the IDE architecture unnecessarily; build on what exists.
- Do not introduce unsupported extension APIs, and do not claim compatibility
  that has not been demonstrated.
- Do not silently install executable extensions; code-bearing installs are
  explicit and warned.
- Do not claim an extension works because installation succeeded.
- Do not use proprietary marketplace endpoints without authorization; use
  documented Open VSX APIs, never page scraping.
- Do not add unnecessary dependencies; no hardcoded credentials.
- Do not claim builds or tests passed without executing them.
- Follow the repository laws in [`AGENTS.md`](../../AGENTS.md): ownership-correct
  changes, direct imports (no barrels), generated-code boundaries respected,
  dead code deleted, design described in our own vocabulary
  ([`scripts/check_design_attribution.py`](../../scripts/check_design_attribution.py)).

---

## 12. Authority and placement

- This README is the user-requested working plan/status record, not a
  canonical behavior spec.
- The [extension system spec](../../specs/systems/extensions/README.md) owns
  implemented behavior and its source map. The system index and frontend area
  doc now link it. Remaining host/install recommendations here are proposals,
  not claims of shipped functionality.
- Run `python3 scripts/check_docs.py` after any documentation change.


### Initial documentation-only verification — 2026-10-12

`python3 scripts/check_docs.py` was executed after this update and exited 1.
It reported five missing-link findings in `specs/DESIGN_SYSTEM.md` and
`specs/systems/workspace-surface/ux-latency-transitions.md`, referencing
`apps/packages/design/src/css/product.css` and `desktop.css`. Both CSS files
were already deleted in the working tree before this update. No finding named
this README. No application build or extension tests were executed in this pass.


## 13. Current implementation slice — discovery and details

Architecture: the shared ProductClient renders a lazy authenticated Extensions
route inside the existing sidebar shell. A scoped registry dependency supplies
TanStack Query access hooks; no client-side installation records are invented.
Existing Button/Input/Select primitives and design tokens supply the controls.
Registry content is treated as untrusted; executable extensions remain blocked.

- [x] Registry access: add abort-aware requests, CORS-safe headers, bounded
  retries, and safe bounded README/changelog/manifest fetching in
  `lib/access/extensions/`; keep pure content URL rules in `lib/domain/extensions/`.
- [x] Surface: add `providers/ExtensionRegistryProvider.tsx`,
  `hooks/access/extensions/use-extension-registry.ts`,
  `hooks/extensions/ui/use-extension-marketplace.ts`, and
  `components/extensions/marketplace/**` for search, pagination, category/sort,
  metadata, README/changelog, version history, dependencies, publisher, and
  manifest capability reporting. Explain unavailable installation rather than
  rendering fake success. Featured/recommendations require an owned source and
  are not fabricated from popularity.
- [x] Integration: add `pages/ExtensionsPage.tsx`, an `APP_ROUTES.extensions`
  entry, lazy routing in `AuthenticatedAppHost.tsx`, and the existing Blocks
  glyph in sidebar navigation. Gate this preview with
  `VITE_EXTENSION_MARKETPLACE=1`, off by default.
- [x] Proof: run the narrow registry/content/UI/route test suites, ProductClient
  typecheck, frontend boundary checks, and docs checks. Record actual results
  and existing-checkout blockers here; do not claim full lifecycle proof.

Observability delta: registry failures render bounded, authored error states.
Registry query hooks mark expected remote failures handled so raw search terms,
metadata, and third-party response content do not enter telemetry. No new
analytics event is introduced in this slice.


### Verification from the implementation pass

- Targeted tests cover public content bounds/URLs, registry mapping/transport,
  UI queries and retry, focus restoration, preview gating, authenticated routing,
  and the existing sidebar. The final run passed 107 tests across 9 files:

  ```bash
  pnpm --filter @proliferate/product-client test src/lib/access/extensions src/lib/domain/extensions src/components/extensions/marketplace src/pages/AuthenticatedAppHost.test.tsx src/pages/ExtensionsPage.test.tsx src/components/workspace/shell/sidebar/MainSidebar.test.tsx --maxWorkers=1
  ```

- `pnpm --filter @proliferate/product-client exec tsc -p tsconfig.json --noEmit`
  passed after updating the sidebar playground caller for its new props.
- `python3 scripts/check_frontend_boundaries.py` and
  `python3 scripts/check_design_attribution.py` passed.
- An isolated Vite production bundle of the marketplace surface passed using
  the existing generated design CSS in `dist`. This is not a full Desktop build.
- `git diff --check` passed.
- The full ProductClient `typecheck` script failed while building design assets:
  the already-deleted `apps/packages/design/src/css/product.css` is required by
  the design package. Its deletion is unrelated to this slice.
- Docs checking still reports the five existing links to deleted design CSS.
- A live documented Open VSX search returned a valid page. No connected browser
  was available for desktop interaction or screenshots; full lifecycle tests
  remain impossible until installation and contribution loading exist.


## 14. Current implementation slice — validated local VSIX installation

Scope: a runtime library installer for local VSIX packages, with no extension
execution or marketplace install control yet. Durable registration and runtime
transport follow this package-level boundary. Installation produces an immutable
package directory and a verified receipt; existing installed versions remain
untouched when preparation fails.

- [x] Harden `adapters/extensions/vsix.rs`: validate all entries before writing,
  reject Windows aliases/duplicate paths, require an empty destination, bound
  manifest reads, and account for actual extracted bytes.
- [x] Add `adapters/extensions/package.rs`: private same-filesystem staging,
  bounded snapshot copy plus SHA-256 verification, cleanup on rejection, and
  locked atomic publication without overwriting existing packages.
- [x] Add `domains/extensions/manifest.rs` and `installer.rs`: validate requested
  identity against the manifest, block executable entrypoints and unsupported
  dependencies/contributions, validate declared resource paths, and return a
  package receipt without claiming activation or API compatibility.
- [x] Execute hostile-archive and installation/rollback tests, run the narrow
  Rust checks and documentation checks, and record their actual results.

The existing agent installer stays unchanged. Its activation helper is scoped to
agent-specific launcher replacement; extension packages use immutable versioned
directories, so preparation failure requires discarding staging rather than
swapping a live executable tree.


### Installation verification — 2026-10-12

- The system `rustc` aborts because Homebrew's LLVM library is missing. Checks
  use the already-installed rustup stable toolchain directly; no system setup
  or repository dependency was changed.
- The original runtime archive tests reproduced four security failures before
  implementation (11 passed, 4 failed). A temporary proof crate imports the
  actual extension adapter/domain source modules with dependency versions pinned
  from this repository's lockfile; its final run passed all 36 tests.
- A separate manual proof downloaded Open VSX's real default-theme VSIX and
  checksum using documented APIs, called the real installer, and verified its
  persisted receipt. No extension code or package script was executed.
- Rust formatting, diff whitespace, AnyHarness boundaries, and the existing
  fence ledger checks passed. The native runtime extension test command also
  passed all 36 tests (2,376 other tests filtered out):

  ```bash
  RUSTC=/Users/michaelshingara/.rustup/toolchains/stable-x86_64-apple-darwin/bin/rustc /Users/michaelshingara/.rustup/toolchains/stable-x86_64-apple-darwin/bin/cargo test -p anyharness-lib extensions:: --lib
  ```

  The first link attempt ran out of disk space. Only incremental cache
  generations created by this task and the temporary proof build target were
  removed before the successful retry. Earlier caches and source files were
  preserved. This is not a full workspace test run or Desktop build.
- The marketplace Install control remains disabled until registration and
  runtime management transport are implemented. No frontend tests were repeated
  for this runtime-only slice.

- `python3 scripts/check_docs.py` was executed after the installer documentation
  changes. It still reports only the five existing links to deleted design CSS;
  no extension document was named.

## 15. Durable registration and removal — implemented and verified

The next runtime slice adds SQLite registration around the verified package
installer. Installation records an intent before publication. Inventory reads
recover interrupted operations by checking the requested identity, receipt,
manifest admission, and declared resources. Registered packages remain inactive:
there is still no contribution loader or executable extension host.

Uninstall records its intent, moves the selected version into a private UUID
quarantine, removes that tree, then deletes its row. Recovery can finish removal
after partial filesystem cleanup or a database failure. Inventory does not adopt
unregistered package directories. Version directories are managed independently.

- [x] Add the additive `0077_installed_extensions` migration and domain store.
- [x] Add blocking manager install/list/uninstall use cases and bounded package
  receipt/manifest reads under a cross-process management lock.
- [x] Add restart, checksum failure, database failure, concurrent install,
  quarantine cleanup, and symlink-tampering lifecycle tests.
- [x] Execute the lifecycle tests and regenerate the schema through its owning
  ignored test; record evidence and run documentation/boundary checks.

Typed runtime transport and marketplace management controls follow this durable
boundary. This slice does not expose client-selected filesystem paths over HTTP
or implement enablement before an activation mechanism exists.

### Registration verification — 2026-10-12

- Native runtime extension tests: **50 passed, 0 failed**, 2,376 unrelated tests
  filtered out. This includes the original 36 package/archive tests and 14 new
  durable lifecycle tests. They reopen the real SQLite database and simulate
  interruptions/faults before publication, after publication, before quarantine,
  after partial cleanup, and after cleanup before the final database write.
- The schema was regenerated through its owning ignored test; the migration
  snapshot equality test then passed. Both commands passed one test each:

  ```bash
  cargo test -p anyharness-lib extensions:: --lib
  cargo test -p anyharness-lib update_anyharness_schema_snapshot --lib -- --ignored
  cargo test -p anyharness-lib anyharness_schema_snapshot_matches_migrations --lib
  ```

  These ran with the installed rustup stable toolchain and explicit `RUSTC`, as
  recorded in Section 14, because the system compiler remains unavailable.
- Initial builds encountered missing generated zstd/SQLite artifacts. Only the
  `libsqlite3-sys` dependency build cache was reset (49 files, 17 MiB). The
  compiler also found an iterator lifetime error in the new store; binding the
  query rows before collecting fixed it. Subsequent native tests passed. No
  source files were removed to address disk pressure.
- Rust 2021 formatting, diff whitespace, AnyHarness boundaries, and the fence
  ledger checks passed. The documentation check still reports only the five
  existing links to deleted design CSS; no new extension documentation error.
- This is verified backend package management. HTTP/SDK transport, marketplace
  controls, updates, workspace enablement, and contribution activation remain
  pending; no complete executable-extension lifecycle is claimed.

## 16. Runtime management API and Installed controls — verified

Connect the verified manager through typed runtime contracts and the generated
SDK. The current runtime owns the list; the client does not invent installation
state. Global extension operations retain the existing runtime authentication
boundary and are not granted to workspace-scoped user claims.

- [x] Add authenticated list and uninstall routes, with blocking filesystem work
  dispatched off the async executor and authored errors that omit private paths.
- [x] Regenerate OpenAPI/SDK types through the owning generator and add SDK methods.
- [x] Connect the Installed section to runtime-scoped queries, package errors,
  inactive status, explicit uninstall confirmation, and post-operation refresh.
- [x] Execute route, SDK, UI, ownership, formatting, and documentation checks.

Registry download/upload, updates, workspace enablement, and contribution
activation remain separate following slices. Discovery continues to work when
there is no connected runtime.

### Management transport verification — 2026-10-12

- Runtime selection `cargo test -p anyharness-lib extensions --lib`: **57 passed,
  0 failed**. This comprises 50 package/manager tests, four new API tests, and
  three existing tests selected by the same substring. The API tests exercise
  real SQLite registration and VSIX files, authenticated inventory, selected
  version removal, idempotent removal, bounded errors, published OpenAPI shapes,
  and denial of global operations to workspace-scoped claims.
- OpenAPI JSON and TypeScript were regenerated using
  `pnpm --filter @anyharness/sdk run generate`, with a freshly compiled runtime.
  Generated changes contain only the two extension routes and their schemas.
  SDK build and contract type checks passed using the installed TypeScript
  binary. SDK transport tests: **2 passed**.
- Product extension access/domain, marketplace, and route selection:
  **89 tests passed across 9 files**, including six Installed management tests.
  Confirmation is required before DELETE; cancellation restores focus; successful
  removal refreshes real inventory; failed removal retains the row. Tests also
  cover absent/older runtimes and separation across runtime and actor changes.
- Direct ProductClient TypeScript checking passed. The first run found an
  exported hook return type referencing a private access-layer interface;
  exporting that interface fixed declaration generation. A final Installed-only
  rerun passed all six tests after the fix and readability edits.
- Rust 2021 formatting, AnyHarness boundaries/fences, frontend boundaries,
  design attribution, and diff whitespace checks passed. Documentation validation
  still reports only five existing links to deleted design CSS files.
- Native compilation used the installed rustup compiler, an isolated target
  under `/tmp/proliferate-extension-api-target`, no incremental compilation, and
  disabled debug information. Missing dependency artifacts and disk pressure
  required removing inactive compiler incremental caches; sources and runtime
  binaries were preserved. The deleted runtime build script prevented the first
  generator build, so its normal development values were supplied for that
  command only: `PROLIFERATE_STAMPED_VERSION=0.1.0` and an empty
  `PROLIFERATE_STAMPED_GIT_SHA`. Local package-runner delays required invoking
  installed check binaries directly.
- Browser/desktop visual qualification and a full workspace build were not run.
  At this point registry installation/upload, updates, enablement, contribution
  loading, and extension execution remained absent; the slice proved inventory
  and removal.

## 17. Registry installation and marketplace outcome — implemented

The runtime install endpoint resolves a requested namespace/name/version through
the configured Open VSX base, downloads the package on the runtime, validates the
registry digest and declarative package policy, and records the verified package.
The client sends only identity; it never downloads package bytes or supplies a
URL/path. The detail view now enables Install when connected, refreshes the
originating actor/runtime's Installed query on settlement, and announces
successful installation as inactive or gives bounded failure copy. No activation
or executable host is implied.

- [x] Add the typed SDK request shape and install transport proof.
- [x] Connect the detail Install action to the runtime and display accessible
  success/failure state; keep it disabled when disconnected.
- [x] Prove runtime-scoped inventory refresh and marketplace outcomes.
- [x] Update the system spec, correct the Installed empty-state copy, and run
  focused client/SDK checks and formatting.

### Installation flow verification — 2026-10-16

- Product marketplace, install-hook, and Installed management tests passed:
  **11 tests across 3 files**. They cover a real POST through the runtime fetch
  seam, inactive success feedback, disconnected disabled state, origin-scoped
  inventory refresh, and existing uninstall behavior.
- SDK extension transport tests passed: **3 tests**, including POST method/body,
  bearer auth, and returned summary shape.
- Some initial runs timed out at Vitest's default 5-second per-test limit under
  parallel load; the same focused suite passed with `--testTimeout=15000`. No
  assertion or production behavior was changed to accommodate the timeout.
- This slice does not add local uploads, updates, activation, workspace
  enablement, contribution loading, or extension execution. Browser/desktop
  visual qualification and the full workspace suite are still unrun.
