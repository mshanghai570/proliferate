# Extensions

Owns Open VSX discovery, extension metadata/content access, manifest compatibility
analysis, local VSIX package materialization, durable registration/removal,
bounded declared app/syntax color-theme reads, and the VSIX inspection/extraction
adapter. The
[workspace surface](../workspace-surface/README.md) owns the shared sidebar and
route shell; [harnesses](../harnesses/README.md) owns agent skills and harness
installation. IDE extensions and agent skills are separate systems.

## Purpose

Let users inspect public registry extensions without implying that they can run
in Proliferate. Desktop includes the discovery preview by default; Web keeps
it opt-in. The runtime
library can materialize and register verified local declarative packages, list
their state, and uninstall selected versions. Typed runtime list/uninstall
endpoints connect the Installed surface. Desktop can copy bounded core colors
from declared JSON theme resources into its appearance preference. No general
contribution loader is connected, and no extension code executes.

## Owned state

The [marketplace hook](../../../apps/packages/product-client/src/hooks/extensions/ui/use-extension-marketplace.ts)
holds search/filter/page/selection state for its mounted screen. TanStack Query
owns remote search, metadata, manifest, and document caches. The scoped
[registry provider](../../../apps/packages/product-client/src/providers/ExtensionRegistryProvider.tsx)
owns an adapter with at most 32 cached ETag responses. The runtime installer
writes immutable version directories and a `.proliferate-package.json` receipt
under the selected runtime home's `extensions/packages` directory. A receipt
records schema version 1, requested identity, and verified SHA-256. It establishes
package materialization, not enablement or API support. SQLite
`installed_extensions` rows own registered identity, checksum, lifecycle state,
bounded error code, and an optional removal UUID. States are `installing`,
`installed`, `removing`, and `error`; all registered packages remain inactive.
Workspace enablement and extension-host processes remain absent.

## Public surface

Desktop enables the sidebar Extensions destination and the
[Extensions page](../../../apps/packages/product-client/src/pages/ExtensionsPage.tsx)
at `/extensions` by default. Set `VITE_EXTENSION_MARKETPLACE=0` in a Desktop
build to hide it. Web continues to require `VITE_EXTENSION_MARKETPLACE=1`.
The authenticated
[route host](../../../apps/packages/product-client/src/pages/AuthenticatedAppHost.tsx)
lazy-loads the page and keeps the workspace mounted while browsing.

[Discovery](../../../apps/packages/product-client/src/components/extensions/marketplace/ExtensionMarketplaceScreen.tsx)
debounces filters for 250 ms, searches 24 results per page, and exposes category,
relevance, download count, rating, and publication-date sorting.
[Details](../../../apps/packages/product-client/src/components/extensions/marketplace/ExtensionDetails.tsx)
show metadata, publisher, dependencies, requirements, README, changelog, versions,
and manifest contributions. Install sends the selected registry identity to the
connected runtime, which resolves and verifies the package before registration.
The UI reports success while clearly stating that the package remains inactive;
failures use bounded authored feedback. Installation remains disabled when no
runtime is connected. The
[Installed section](../../../apps/packages/product-client/src/components/extensions/marketplace/InstalledExtensions.tsx)
loads the connected runtime's registered versions on demand, offers **Details and
versions** for the package's current registry listing, shows inactive state
and package errors, and requires confirmation before uninstall. Missing runtime
connections and unsupported management endpoints show authored messages instead
of a fabricated empty list. Queries include runtime URL and actor cache scope;
uninstall completion refreshes the originating scope, even after target changes.
Returning from registry details preserves the Installed destination and restores
keyboard focus to its tab. Browsing details does not install or remove a version.

On Desktop, **App color themes** reads declared theme data on demand. A readable
palette can be copied into the existing imported app color preference; this
snapshot survives package removal and resets through Appearance. The action
does not enable an extension host or syntax theme. The authenticated
`GET /v1/extensions/{namespace}/{name}/{version}/color-themes` endpoint verifies
registration and package validity under the management lock, then reads at most
32 declared theme files, each bounded to 1 MiB. It returns only a bounded label,
normalized light/dark mode, six supported color fields, availability, and a
truncation flag. Standalone JSON and JSONC themes are supported; includes are
never followed. The client validates color syntax and text contrast before
offering the copy action. Queries retain runtime and actor isolation.

On Desktop, **Syntax themes** reads the exact installed version through
`GET /v1/extensions/{namespace}/{name}/{version}/syntax-themes`. It applies a
validated preference snapshot containing foreground rules and matching app
background/text colors. Source views, transcript code blocks, and code diffs use the existing
syntax highlighter with the imported theme object. Selection survives restart
and uninstall; Appearance resets syntax independently from app colors. The
selection applies only in its matching light/dark mode. Markdown uses its grammar
when imported syntax colors are selected; built-in Markdown rendering is unchanged.
The same receipt, resource, registration, owner-authentication, and management-lock
checks as app color reads apply. At most 32 themes, 256 rules per theme, 32 scopes
per rule (at most 1,024 UTF-8 bytes each), and 32 KiB of UTF-8 scope/color data per theme are admitted. Foreground
colors must be six-digit hex. Includes, token-file paths, semantic token colors, and font styles are not
loaded. Theme resource parsing accepts line/block comments, trailing commas,
and a UTF-8 BOM before validating values with the existing JSON parser. Package
manifests and receipts remain strict JSON. Syntax application copies only editor
background/text colors, so optional UI colors with alpha do not block syntax. The client revalidates preferences
and app-color contrast. Highlight cache accounting includes custom theme data,
so switching themes cannot bypass its existing memory budget. Theme data never
executes code or registers additional editor APIs.

The blocking library entrypoint
[install_local_vsix](../../../anyharness/crates/anyharness-lib/src/domains/extensions/installer.rs)
accepts the trusted runtime home, local archive path, requested namespace/name/
version, and a required SHA-256. It validates and snapshots the archive, admits
only declared data contributions, verifies resource existence, writes a receipt,
and publishes a new version directory. Existing versions are never overwritten.
Runtime transports must dispatch this filesystem work off their async executor.
The blocking [manager](../../../anyharness/crates/anyharness-lib/src/domains/extensions/manager.rs)
wraps installation with a durable intent, lists registered packages, and removes
selected versions. Every manager operation holds a cross-process management lock
and reconciles unfinished rows before proceeding. Inventory rechecks receipts,
manifest admission, and resource existence; it does not rehash every extracted
resource or enumerate/adopt unregistered directories. Missing or invalid packages
produce a bounded error state. The [HTTP adapter](../../../anyharness/crates/anyharness-lib/src/api/http/extensions.rs)
exposes `GET /v1/extensions`, `POST /v1/extensions/install`, and
`DELETE /v1/extensions/{namespace}/{name}/{version}`.
[App wiring](../../../anyharness/crates/anyharness-lib/src/app/mod.rs) constructs the
manager with the same runtime home and SQLite connection as the application.
The adapter runs blocking use cases off the async executor and maps results into
[public contracts](../../../anyharness/crates/anyharness-contract/src/v1/extensions.rs).
Responses omit installation directories and removal UUIDs; expected errors use
bounded codes and authored descriptions rather than filesystem error text.
The existing runtime bearer boundary applies. Workspace-scoped user claims are
not granted these global operations. The runtime's registry install route accepts
only namespace/name/version, resolves package metadata and downloads server-side,
then verifies the checksum and package admission before registration. Clients do
not supply filesystem paths or download URLs. There is no native file-picker or
client-upload route.

## Consumes

The [client registry adapter](../../../apps/packages/product-client/src/lib/access/extensions/open-vsx-registry.ts)
implements the [discovery port](../../../apps/packages/product-client/src/lib/domain/extensions/extension-models.ts)
using the documented [Open VSX API](https://github.com/eclipse-openvsx/openvsx/wiki/Registry-API).
Search calls `/api/-/search`; metadata calls `/api/{namespace}/{name}`; version
metadata supplies registry details. The runtime's separate registry adapter
resolves/downloads package bytes server-side for installation; the client only
sends the extension identity.

The [manifest parser](../../../apps/packages/product-client/src/lib/domain/extensions/extension-manifest.ts)
extracts declared contributions and entrypoints. The
[compatibility analyzer](../../../apps/packages/product-client/src/lib/domain/extensions/extension-compatibility.ts)
compares declarations with an empty host capability set. This is preliminary
analysis, not a demonstrated compatibility guarantee or an installation gate.

## Invariants

- **Installation does not execute an extension.** The detail surface sends only
  an identity to the runtime install mutation; it has no extension-code loader.
- **Theme reads expose declared color data only.** Registered package receipts,
  manifests, and resource paths are validated before reading. Unregistered or
  damaged packages cannot supply themes. The endpoint omits paths, arbitrary
  JSON fields, and resource error text; unsupported files remain unavailable.
- **Untrusted content cannot dispatch IDE commands.** The
  [URL rule](../../../apps/packages/product-client/src/lib/domain/extensions/extension-content.ts)
  accepts HTTPS URLs without embedded credentials. The
  [document renderer](../../../apps/packages/product-client/src/components/extensions/marketplace/ExtensionDocument.tsx)
  skips raw HTML and rejects local-file, command, script, and data URLs.
- **Public requests omit application cookies.** Registry and content fetches
  use `credentials: "omit"` and accept cancellation signals. Text reads are
  bounded to 1 MiB for documents/manifests and 2 MiB for registry responses in
  [content access](../../../apps/packages/product-client/src/lib/access/extensions/extension-content.ts).
- **Registry retries are bounded.** The adapter retries a 429 twice and honors
  `Retry-After` only when waiting at most 30 seconds per attempt; longer delays
  return a typed rate-limit failure for an explicit later retry.

- **Publication follows verified private staging.** The
  [package adapter](../../../anyharness/crates/anyharness-lib/src/adapters/extensions/package.rs)
  copies the source into its own UUID staging directory while hashing it,
  validates the expected digest, and extracts only that snapshot. The source
  file can change afterward without changing the published package.
- **Declared executable entrypoints are refused.** The runtime
  [manifest policy](../../../anyharness/crates/anyharness-lib/src/domains/extensions/manifest.rs)
  rejects `main` and `browser`, dependencies/extension packs, and contributions
  outside themes, icon themes, grammars, snippets, and language declarations.
  Admission does not claim engine-range or API compatibility. Package scripts
  are metadata only; the installer never invokes a package manager or process.
- **The archive cannot overwrite existing destination files.**
  [VSIX preflight](../../../anyharness/crates/anyharness-lib/src/adapters/extensions/vsix.rs)
  rejects traversal, Windows aliases, symlinks, and normalized/case-folded
  duplicate paths before writing. Extraction requires an empty destination and
  uses exclusive file creation. Manifests are bounded to 1 MiB; compressed
  packages to 128 MiB; expansion to 4,096 entries, 32 MiB per entry, 256 MiB total.
- **Failed publication preserves existing versions.** The package adapter
  holds an exclusive install lock, refuses an existing target, syncs staging,
  and atomically renames it on the same filesystem. A failed parent-directory
  sync renames the new package back into staging for cleanup. A rollback I/O
  failure is returned explicitly rather than reported as installation success.

- **Interrupted management operations retain their intent.** SQLite records an
  install before publication and marks it installed afterward. A matching
  published receipt permits recovery after a failed final database write. An
  unpublished interrupted install becomes an error, never an active extension.
- **Uninstall only removes the registered version.** After checking the receipt,
  the manager records a removal UUID, renames the version into private
  quarantine, removes that tree, syncs its parent, then deletes its row. Recovery
  can finish partially cleaned quarantine without needing its removed receipt.
  Unregistered directories and mismatched receipts are not adopted or deleted.

## Emits

The [access hooks](../../../apps/packages/product-client/src/hooks/access/extensions/use-extension-registry.ts)
mark expected registry failures handled. The
[failure surface](../../../apps/packages/product-client/src/components/extensions/marketplace/ExtensionRegistryFailure.tsx)
shows authored messages by bounded failure code and a retry action. No new
analytics event is emitted; registry bodies and search terms are not submitted
as exception payloads by these hooks. The local installer uses a `skip_all`
use-case tracing span. Staging cleanup failure emits only the bounded
`extension_staging_cleanup_failed` classification, without paths or manifest
content.

## Fences

The client does not own package files or durable installation state. The runtime
[VSIX adapter](../../../anyharness/crates/anyharness-lib/src/adapters/extensions/vsix.rs)
reads manifests and extracts archives for the
[local installer](../../../anyharness/crates/anyharness-lib/src/domains/extensions/installer.rs).
The [package adapter](../../../anyharness/crates/anyharness-lib/src/adapters/extensions/package.rs)
owns snapshot hashing, filesystem limits, private staging, locking, publication,
and cleanup. The domain owns manifest admission and receipt meaning. Installation
has no startup activation or account synchronization side effect.
The [management adapter](../../../anyharness/crates/anyharness-lib/src/adapters/extensions/management.rs)
owns bounded receipt/manifest reads, lifecycle locking, quarantine, and removal.
The [domain store](../../../anyharness/crates/anyharness-lib/src/domains/extensions/store.rs)
owns registration queries. Recovery runs on explicit manager operations; it does
not start a host or scan packages at application startup.
Runtime structure follows [AnyHarness](../../areas/anyharness.md); client layering
follows [frontend standards](../../areas/frontend.md).

## Code map

The [SDK client](../../../anyharness/sdk/src/client/extensions.ts) exposes typed
list/install/uninstall methods using the runtime's generated OpenAPI types. Product
[access](../../../apps/packages/product-client/src/lib/access/anyharness/extensions.ts)
resolves the inherited runtime connection; query and mutation hooks own cache
policy, cancellation, and refresh. UI hooks own confirmation and focus restoration.


```text
apps/packages/product-client/src/
├── pages/ExtensionsPage.tsx                   gated route entry
├── providers/ExtensionRegistryProvider.tsx    scoped registry dependency
├── components/extensions/marketplace/         discovery, details, public content
├── hooks/extensions/ui/                       search/page/selection and detail state
├── hooks/access/extensions/                   registry and runtime management queries
├── lib/access/extensions/                     registry transport and bounded text reads
├── lib/domain/extensions/                     wire mapping, manifests, compatibility, URLs
│   └── extension-color-theme.ts               readable core palette admission
├── lib/domain/capabilities/extension-marketplace.ts   preview gate
├── config/extensions/                         categories and page size
└── copy/extensions/                           authored status/error copy
anyharness/crates/anyharness-lib/src/adapters/extensions/
├── mod.rs                                    adapter registration
├── vsix.rs · vsix_tests.rs                    bounded archive validation/extraction
├── package.rs · package_tests.rs              snapshot, integrity, staging, publication
├── management.rs                             lock, bounded reads, quarantine/removal
└── theme_json.rs                             theme-only JSONC normalization
anyharness/crates/anyharness-lib/src/domains/extensions/
├── mod.rs                                    domain registration
├── manifest.rs                               identity and declarative admission policy
├── installer.rs · installer_tests.rs          local install use case and receipts
├── model.rs · store.rs                        registered lifecycle state and SQLite access
├── manager.rs · manager_tests.rs              install/list/remove and interruption recovery
├── themes.rs                                 bounded declared app color reads
└── syntax_themes.rs                          bounded foreground scope/color rules
anyharness/crates/anyharness-lib/src/persistence/
└── sql/0077_installed_extensions.sql          additive registration migration
anyharness/crates/anyharness-contract/src/v1/extensions.rs   public wire models
anyharness/crates/anyharness-lib/src/api/http/extensions.rs  runtime transport
anyharness/crates/anyharness-lib/src/api/extensions_tests.rs real-router proofs
anyharness/sdk/src/client/extensions.ts                    typed list/uninstall
anyharness/sdk/src/types/extensions.ts                     generated type aliases
```

## Proof

- Registry mapping/transport, cancellation, bounded content, and hostile URL
  tests are colocated under
  [registry access](../../../apps/packages/product-client/src/lib/access/extensions/)
  and [domain rules](../../../apps/packages/product-client/src/lib/domain/extensions/).
- [Surface tests](../../../apps/packages/product-client/src/components/extensions/marketplace/ExtensionMarketplaceScreen.test.tsx)
  exercise queries, filters, pagination, metadata, README safety, compatibility
  reporting, and retry using deterministic registry responses.
- [Route tests](../../../apps/packages/product-client/src/pages/AuthenticatedAppHost.test.tsx)
  prove the workspace remains mounted while the Extensions route is active.
- [Archive tests](../../../anyharness/crates/anyharness-lib/src/adapters/extensions/vsix_tests.rs)
  cover traversal/Windows aliases, duplicates, symlinks, destination refusal,
  manifest bounds, and extraction limits.
- [Package tests](../../../anyharness/crates/anyharness-lib/src/adapters/extensions/package_tests.rs)
  cover private permissions, checksum/CRC rejection, compressed-size limits,
  snapshot independence, concurrent publication, staging symlink refusal, and
  rollback after a failed publication sync.
- [Installer tests](../../../anyharness/crates/anyharness-lib/src/domains/extensions/installer_tests.rs)
  cover manifest identity, executable/dependency/contribution admission,
  resource paths/existence, receipt spoofing, and preservation of existing
  versions. These are materialization proofs, not a complete extension lifecycle.

- [Manager tests](../../../anyharness/crates/anyharness-lib/src/domains/extensions/manager_tests.rs)
  reopen real SQLite state and cover registration, selected-version uninstall,
  checksum failure, concurrent managers, mismatched receipts, symlink tampering,
  and recovery across install/removal intent boundaries and failed database
  writes. Unregistered directories remain untouched and packages remain inactive.
- The generated [SQLite schema](../../areas/anyharness-db-schema.sql) is pinned by
  `anyharness_schema_snapshot_matches_migrations`; regeneration uses its owning
  ignored `update_anyharness_schema_snapshot` test.
- [Runtime API tests](../../../anyharness/crates/anyharness-lib/src/api/extensions_tests.rs)
  exercise real registration, authenticated listing, selected-version removal,
  sanitized validation errors, OpenAPI registration, and denial of global
  management to workspace-scoped claims.
- [SDK tests](../../../anyharness/sdk/src/client/extensions.test.ts) cover runtime
  authentication, cancellation forwarding, identity encoding, install request
  shape, and empty removal responses.
- The product installation hook test proves the successful install mutation
  refreshes the Installed inventory for the originating runtime and actor scope.
  Marketplace tests cover connected-runtime installation, user-visible inactive
  success, and the disconnected disabled state.
- [Installed surface tests](../../../apps/packages/product-client/src/components/extensions/marketplace/InstalledExtensions.test.tsx)
  exercise inactive status, confirmation and focus restoration, failed removal,
  disconnected/older runtimes, and separation across runtime and actor changes.
- [Installed color tests](../../../apps/packages/product-client/src/components/extensions/marketplace/InstalledExtensionColors.test.tsx)
  exercise explicit application, unreadable palette refusal, actor changes, and
  bounded failure copy. Runtime manager and API tests prove registered theme
  reads, include refusal, bounded inventory, authentication, and response shape.

## Current gaps

- Registry installation is implemented for the connected runtime, but registry
  download/upload from a local client, updates, workspace enablement,
  general contribution loading, and executable-host isolation are absent. Packages remain
  inactive; the [detail surface](../../../apps/packages/product-client/src/components/extensions/marketplace/ExtensionDetails.tsx)
  communicates that boundary and shows bounded success/failure outcomes.
- Declared color themes supply only core app colors through an explicit copy.
  Syntax foreground colors are available through an explicit copy of supported
  installed JSON/JSONC themes. Includes, token font styles, semantic token
  overrides, and theme asset loading are absent.
  Only the first 32 declared themes are offered; truncation is visible.
- Featured and recommended collections have no owned source. Discovery exposes
  real registry sorting without relabeling popularity as recommendations.
- [Compatibility analysis](../../../apps/packages/product-client/src/lib/domain/extensions/extension-compatibility.ts)
  does not detect JavaScript API usage; engine-range/platform helpers are partial
  and platform-specific download selection is not wired into registry access.
- [Package installation](../../../anyharness/crates/anyharness-lib/src/domains/extensions/installer.rs)
  requires an expected checksum supplied by its caller; the runtime registry
  adapter currently obtains that digest from Open VSX. Publisher-signature
  verification and dependency resolution are not implemented. A matching
  checksum is integrity evidence, not publisher authentication.
- [Package staging](../../../anyharness/crates/anyharness-lib/src/adapters/extensions/package.rs)
  cleans rejection during normal execution; a process crash can leave private
  staging residue. No startup sweep exists. Private Unix modes protect against
  other users, not a malicious process with the same account. Windows inherits
  runtime-home ACLs and has no directory-fsync primitive in this adapter.

Observable failures: registry/content 404 returns `not-found`; 429 returns
`rate-limited` after bounded retries; other non-success responses return
`http-error`; invalid JSON/content limits return `malformed-response`; failed
fetches return `network-error`. Cancellation propagates to the query consumer.
The failure surface offers retry; absent publisher documents show an explicit
unavailability message.

Local installation additionally reports typed invalid checksum/layout/identity,
checksum mismatch, archive violation, unsupported entrypoints/dependencies/
contributions, missing resources, existing-version, and I/O/rollback failures.
The caller receives an error; the installer does not enable a partial package.

Syntax theme proofs are colocated in `domains/extensions/syntax_themes.rs`,
manager/router tests, the SDK extension client test, imported syntax preference
tests, the real highlighter test, and `InstalledExtensionSyntax.test.tsx`. They
cover bounded admission, includes/symlink refusal, owner access, safe response
shape, real scope colors, concurrent palette isolation, cache identity, explicit
application/reset, preference persistence, and actor changes. No new telemetry
event is emitted; authored query failures remain telemetry-handled and theme
contents are not logged.
