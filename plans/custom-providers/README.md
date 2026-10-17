# Custom Providers Implementation Plan

**Goal:** Configure arbitrary OpenAI-compatible endpoints for local OpenCode sessions from Settings.

**Architecture:** Store each connection as an encrypted `openai_compatible` vault entry. Deliver it through the existing typed provider-config source, using OpenCode's `OPENCODE_CONFIG_CONTENT` environment input. Compose multiple connections in the runtime and let the existing target probe observe executable models.

**Tech stack:** Python/FastAPI/SQLAlchemy, Rust/serde, React/TypeScript, generated Cloud SDK.

**Scope approval:** The user approved the three-stage proposal in chat on 2026-10-16, starting with custom providers. This checkout now contains the custom-provider work and subsequent custom ACP registration and skill-management implementations; validate each against its owning requirements before treating it as complete.

## Constraints

- Keep unrelated working-tree changes intact.
- Preserve native provider logins and existing source selections.
- Keep credentials encrypted in the vault and out of logs and display responses.
- Support endpoints with no key, including loopback HTTP endpoints.
- Use the supplied API base URL exactly; do not append `/v1`.
- Register model IDs with OpenCode; only target-observed IDs enter executable pickers.
- Expose configuration for local OpenCode only in this stage.
- Generate SDK contracts through `make cloud-client-generate`.

## Tasks

- [x] Add server validation, typed vault kind and migration, registry declaration, and state rendering. Required fields: `name`, `endpoint`, `models` (one ID per line); optional fields: `apiKey`, `apiFormat` (`chat` or `responses`). Reject malformed URLs and interpolation directives. Provider IDs derive from vault IDs. Verify encrypted storage and safe state rendering with focused tests.
- [x] Compose OpenCode provider-config inputs in Rust without dropping sibling connections. Reject duplicate provider IDs and malformed input with bounded errors. Protect the configuration input from launch-layer overrides. Verify additive composition and fail-closed handling.
- [x] Generate the Cloud SDK and add the typed create operation. Preserve typed selections through the auth editor. Add a local OpenCode connection form and enabled/remove controls through access and workflow hooks. Verify save/bind rollback and selection preservation.
- [x] Update the owning agent-auth and harness documentation, run the documentation checker, focused tests, and type/build checks. Report any unavailable live-provider verification explicitly.

## Follow-on stages

Custom ACP registrations and skill management were started after this provider slice. Their code and UI are now present in the working tree, but were not yet included in this plan's original acceptance criteria or verification. Track verification and remaining work in the sections below rather than describing either as absent.

## Additional capabilities found in this checkout

### Custom ACP agents

- Settings exposes runtime-local custom ACP commands with separate executable,
  literal-argument, and environment inputs; model discovery uses the existing
  launch-option probe. Registrations are excluded from managed distribution and
  gateway auth. Review target-observed model/controls, safe removal behavior,
  malformed-registration recovery, and local-only access before declaring this
  capability complete.
- Current focused UI test evidence: custom-provider tests do not cover ACP. The
  custom-agent form/component and runtime lifecycle tests still need a specific
  test run. The `anyharness-lib` targeted `custom` test compilation was attempted
  but stopped after prolonged compilation under local resource pressure; it did
  not produce a test result.

### Skills

- Settings imports one folder's files, controls a user default or repo-root
  override, and removes packages. The runtime stores packages locally and
  materializes enabled instructions/resources for later launches. Skills remain
  advisory: they do not grant tools, permissions, MCP servers, or approvals.
- Runtime adapter/domain/API/client files exist, and the feature is described in
  [`specs/systems/harnesses/skills.md`](../../specs/systems/harnesses/skills.md).
  Focused skill UI/runtime tests were not discovered in the current tree; add or
  locate proofs for import path validation, override semantics, launch instruction
  delivery, hostile workspace symlinks, and removal before calling this complete.

### Cross-feature verification snapshot — 2026-10-17

- Server custom-provider validation/rendering tests: **14 passed** with
  `DEBUG=true`; running without debug failed during settings import because the
  local environment omitted production-required JWT/cloud secrets.
- Client custom-provider modal, connection section, and selection workflow:
  **5 passed**. This is not evidence for ACP or skills UI.
- `cargo +stable test -p anyharness-lib custom --lib` compiled dependencies but
  the large `anyharness-lib` test target remained in rustc compilation for more
  than 13 minutes under local resource pressure. It was stopped without a result.
- `cargo +stable fmt --all -- --check` reports formatting differences across
  pre-existing and feature files; the complete-worktree check is not a clean
  proof for these feature changes. No unrelated formatting was applied.
- AnyHarness SDK `pnpm exec tsc --noEmit` was run and fails with seven missing
  generated-schema properties: `CustomAcpAgent`, `RegisterCustomAcpAgentRequest`,
  `SkillFile`, `SkillSummary`, `ImportSkillRequest`, `SetSkillEnabledRequest`,
  and `ListSkillsResponse`. The generated OpenAPI JSON/TS also omit the
  corresponding custom-agent and skill routes. Rust OpenAPI source registers
  those operations and schemas, but its generated artifacts are stale.

- In this continuation, `pnpm exec tsc --noEmit` for AnyHarness SDK fails on
  the seven missing OpenAPI schema properties above. The supported schema
  generator (`CARGO_INCREMENTAL=0 cargo run --bin anyharness -- print-openapi`)
  timed out after 600 seconds without output; free disk fell from roughly 2.3 GB
  to under 1 GB, so further Rust compilation was stopped. No hand-written
  generated artifacts were added. The focused runtime compilation remains
  incomplete, and no focused ACP/skills UI tests or live smoke tests were found
  or run. `git diff --check` passes. The docs check via `uv run python
  scripts/check_docs.py` still fails on five existing links to deleted design
  CSS files.
- The supported generator (`cargo run --bin anyharness -- print-openapi`) was
  attempted with `CARGO_INCREMENTAL=0` and timed out after 10 minutes without
  producing JSON. Available disk fell from about 2.3 GB to 1.0 GB during the
  attempt, so no second full Rust build was started. Generated OpenAPI artifacts
  were not hand-edited; schema/code generation remains blocked pending adequate
  build resources.
- Focused runtime test compilation previously remained in rustc for over 13
  minutes and was stopped without a result. No focused custom ACP or skills UI
  tests were found under the Product Client test tree; no new UI/runtime tests
  or live smoke test were run in this continuation.
- `git diff --check` passed after the tracked source/contract changes.
- Documentation validation remains blocked by five existing references to
  deleted shared design CSS files.
- No UI or live ACP/skill runtime smoke test has been performed. These features
  are present/started, not yet end-to-end verified.

## Verification

Current readiness check (2026-10-17):

- Restored the two required design stylesheets and three Rust binary build
  scripts after user approval. Other unrelated deletions remain untouched.
- The normal runtime OpenAPI generator passed without temporary build metadata.
  ACP/skills contracts are generated through the supported tool.
- Both Rust workspace and standalone runtime builds passed using installed
  stable Rust 1.99.0. The repository's pinned Rust 1.98.0 was not used for these
  readiness builds.
- All six profile frontend packages built successfully. Desktop TypeScript
  checking passed; all native icon paths resolve.
- Fresh focused checks passed: 14 server provider tests and 19 frontend tests
  (5 provider, 8 ACP form, 4 skill import, 2 disconnected-settings regressions).
- Isolated runtime HTTP smoke checks passed for ACP CRUD, environment redaction,
  readiness, skills import/enablement/validation, private persistence, restart,
  and removal. A deterministic ACP subprocess verified model discovery, launch,
  first-prompt skill delivery, binary resources, resumed-session delivery, and
  repository overrides without real LLM credentials.
- The pinned OpenCode binary discovered a configured custom model in an
  isolated home. Real provider inference with user credentials remains untested.
- Fixed disconnected-runtime hooks and per-process skill delivery during review.
  Dynamic agent identities remain bounded in telemetry classification.
- Regenerated the provider logo map through its owning tool using only retained
  assets. All 141 imports and all 11 root README image references resolve.
- Installed private local PostgreSQL 16 and Redis 7 tools from checksum-verified
  official sources. Created the isolated `tryout` database and applied all
  migrations through `c7a10e162026` successfully.
- API/runtime health, hosted Web, and Desktop renderer return HTTP 200.
  The browser renders the first-run setup form. Account creation is left to the
  user; the setup token and local signing/encryption secrets remain private.
- The native profile development build passed. `Proliferate (tryout)` launched
  as a native x86_64 process and logged a healthy connection to its runtime.
  Browser checks verified the setup form, home screen, and sign-in form render.
  Native window interaction and authenticated provider inference were not tested.
- Live-profile ACP and skills APIs return HTTP 200. Local settings, setup token,
  and startup log permissions are `0600`. `make dev-list` reports `tryout` running.
- Claude and Codex managed adapters failed to install under npm 11.19.0 with the
  user's existing `allow-scripts` policy (`EALLOWSCRIPTS`, upstream npm issue
  9783). The npm policy was preserved. OpenCode, Cursor, and Grok installed;
  they require provider credentials or login before use. This compatibility
  limitation is documented in the root README.

The root README documents provider, ACP, and skill setup plus prerequisites,
private local settings, the explicit single-org first-run launch, and account
setup. The machine-local trial launcher lives outside the repository at
`~/.proliferate-local/start-tryout.sh`.
