<div align="center">

<p>
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="./assets/readme/proliferate-wordmark-dark.svg" />
    <img src="./assets/readme/proliferate-wordmark-light.svg" width="320" alt="Proliferate" />
  </picture>
</p>

<h3>The open-source AI IDE</h3>

<p>
  <a href="https://github.com/proliferate-ai/proliferate/stargazers"><img alt="GitHub stars" src="https://img.shields.io/github/stars/proliferate-ai/proliferate?style=flat&amp;logo=github&amp;label=stars" /></a>
  <a href="https://proliferate.com/changelog"><img alt="Latest release" src="https://img.shields.io/badge/release-changelog-0969DA?style=flat" /></a>
  <a href="./LICENSE"><img alt="License" src="https://img.shields.io/badge/license-AGPL--3.0-blue?style=flat" /></a>
  <a href="https://proliferate.com/docs"><img alt="Docs" src="https://img.shields.io/badge/docs-view-0969DA?style=flat" /></a>
  <a href="https://proliferate.com"><img alt="Website" src="https://img.shields.io/badge/website-visit-0969DA?style=flat" /></a>
  <a href="https://discord.gg/2RVNNzEZnj"><img alt="Discord" src="https://img.shields.io/badge/discord-join-5865F2?style=flat&amp;logo=discord&amp;logoColor=white" /></a>
</p>

<br />

Run Claude Code, Codex, OpenCode, Cursor, Grok, and custom ACP agents in parallel, in one workspace.<br />
Each task gets an isolated git worktree for its branch, terminal, conversation, and review state.

<br />

<p>
  <a href="https://proliferate.com"><strong>Download for macOS</strong></a>
  &nbsp;&bull;&nbsp;
  <a href="https://proliferate.com/docs">Documentation</a>
  &nbsp;&bull;&nbsp;
  <a href="https://proliferate.com/changelog">Changelog</a>
  &nbsp;&bull;&nbsp;
  <a href="https://discord.gg/2RVNNzEZnj">Discord</a>
</p>

<img width="full" alt="Proliferate" src="./assets/readme/hero.png" />

</div>

## Features

- 🤖 **[Native harnesses](https://proliferate.com/docs/product/agents)** - Claude Code, Codex, OpenCode, Cursor, Grok, and more
- 🔌 **Custom AI providers** - connect OpenAI-compatible endpoints to local OpenCode sessions, including providers that run on your machine
- 🛠️ **Custom ACP agents** - connect installed agents with their own command, arguments, and environment
- 📚 **Agent skills** - import `SKILL.md` packages and resources, then enable them for all repositories or individual repositories
- 🌳 **[Worktree workspaces](https://proliferate.com/docs/product/workspaces)** - an isolated branch and working directory for every task
- 🔀 **[Parallel agents](https://proliferate.com/docs/concepts/parallel-agents)** - run agents side by side in the same workspace, each on its own task
- 🪆 **[Subagents](https://proliferate.com/docs/features/subagents)** - agents delegate scoped work to child agents and pick the results back up when they finish
- 🧩 **[Integrations](https://proliferate.com/docs/product/integrations)** - connect MCP servers and supported services, then make their tools available to agent sessions through Proliferate's integration gateway
- 🧱 **Extensions marketplace (preview)** - browse Open VSX extensions and install supported declarative packages when the preview is enabled; installed packages remain inactive and do not execute
- ⏰ **[Workflows](https://proliferate.com/docs/product/workflows)** - recurring and event-driven agent runs: nightly review passes, triage on alerts, dependency bumps

### Current customization scope

Custom AI providers are currently configured for local OpenCode sessions; they
do not create general-purpose agents that can be selected across every harness.
Custom ACP agents are a separate option: they must be installed locally and
implement Agent Client Protocol. See the setup sections below.

The Extensions page is an opt-in preview, enabled in a build with
`VITE_EXTENSION_MARKETPLACE=1`. It provides Open VSX discovery and installation
of a limited set of declarative package contributions. Installed extensions are
registered but inactive: Proliferate does not load extension code or activate
themes, icon themes, grammars, snippets, or language declarations.

Appearance settings cover the built-in color modes and text sizing, plus an
imported theme: **Settings → Appearance** accepts a VSCode-compatible theme
JSON file, and its background, surface, foreground, accent, selection, and
terminal palette apply while the matching color mode is active. Roles the
theme does not define keep the built-in Mono palette, and editor syntax
colors are not yet themed. Proliferate does not currently support selecting
custom fonts from macOS, theming editor grammars from `tokenColors`, or
project RAG and semantic indexing.

### macOS product direction

The product direction is a complete coding workspace in the macOS app. The
capabilities below are goals, not claims about what is available today. Current
behavior is described in the sections above and in the linked product docs.

- **A first-class extension marketplace:** discover, install, update, enable,
  disable, and remove editor extensions from one in-app catalog. Extensions
  should integrate with the running editor, including themes, language support,
  snippets, and other supported contributions, with clear compatibility and
  permission information.
- **Built-in multi-agent orchestration:** create dual-agent setups and larger
  agent swarms from the app, assign work on the same project, review results,
  and coordinate changes. Each agent should have its own harness, provider,
  model, and instructions. For example, one agent could use a model configured
  through OpenCode while another uses Codex, in the same setup. This should be
  a native product workflow, not something users have to assemble from agent
  skills.
- **First-class provider and model configuration:** configure compatible model
  providers and select them per agent, alongside native harnesses and ACP
  agents. Provider choice should not be tied to one harness or one global
  setting.
- **Project knowledge with RAG:** index project content, retrieve relevant
  context for an agent's task, and let users inspect and control what is
  indexed and sent to a model.
- **Built-in MCP management:** add and manage MCP servers, inspect available
  tools, and control which agents or projects can use them.
- **Personalized editor appearance:** additionally theme editor syntax
  colors (beyond the shell-color import already shipped), choose custom fonts
  installed on macOS, and adjust editor appearance from the app.

These capabilities can ship in stages. README capability claims should be
updated as each part becomes usable in the macOS app.

### Daily-driver guide

This is the working order for making Proliferate a complete local coding
workspace. The accepted campaign is deliberately staged: make external tools
usable, activate installed extensions safely, then add project knowledge.

| Area | Where we are | Next |
| --- | --- | --- |
| Marketplace | Open VSX discovery and install are available in the opt-in preview. Installed packages are registered but inactive. | PR2: build the extension activation host and enable supported contributions. |
| MCP | Product MCP integrations already provide Proliferate tools. User-managed stdio and HTTP servers are implemented in Settings, the runtime API, and future session launches; generated OpenAPI/SDK artifacts and focused runtime/client checks are complete. | Pass the product acceptance gate before calling PR1 complete. |
| Project knowledge | Project RAG and semantic indexing are not shipped. | PR3: add codebase indexing and retrieval with user-visible control over indexed and supplied context. |
| Skills | Runtime-local `SKILL.md` packages can be imported and enabled by default or repository override. | Continue improving this shipped capability as needed; it is not blocked on the MCP campaign. |
| ACP agents | Local custom ACP registrations can be added and launched when the agent supports model discovery. | Broader distribution and per-agent configuration are not part of PR1–PR3. |
| Model providers | Custom OpenAI-compatible providers work for local OpenCode sessions. | General provider/model selection across harnesses remains a separate design decision. |

PR1's implementation checklist is in
[`docs/superpowers/plans/2026-10-26-user-mcp-servers.md`](./docs/superpowers/plans/2026-10-26-user-mcp-servers.md);
its frozen scope and acceptance gate are in
[`delivery/daily-driver/delivery-spec-daily-driver-pr1-user-mcp-servers.md`](./delivery/daily-driver/delivery-spec-daily-driver-pr1-user-mcp-servers.md).
The system owners are [harnesses](./specs/systems/harnesses/README.md),
[extensions](./specs/systems/extensions/README.md), and
[settings](./specs/systems/settings/README.md). As of 2026-10-26, the
implementation, generated contracts, and focused Rust, SDK, and Settings
checks are complete in the worktree. The manual product acceptance gate is
still pending; do not treat PR1 as complete until it passes.

## Supported agents

Proliferate runs each agent through its native harness.

<table>
  <tr>
    <td align="center" width="120">
      <img src="./apps/desktop/public/provider-icons/claude.svg" width="40" height="40" alt="Claude" /><br />
      <strong>Claude</strong>
    </td>
    <td align="center" width="120">
      <picture>
        <source media="(prefers-color-scheme: dark)" srcset="./apps/desktop/public/provider-icons/openai-dark.svg" />
        <img src="./apps/desktop/public/provider-icons/openai.svg" width="40" height="40" alt="Codex" />
      </picture><br />
      <strong>Codex</strong>
    </td>
    <td align="center" width="120">
      <img src="./apps/desktop/public/provider-icons/opencode.png" width="40" height="40" alt="OpenCode" /><br />
      <strong>OpenCode</strong>
    </td>
    <td align="center" width="120">
      <picture>
        <source media="(prefers-color-scheme: dark)" srcset="./assets/readme/cursor-dark.svg" />
        <img src="./apps/desktop/public/provider-icons/cursor.svg" width="40" height="40" alt="Cursor" />
      </picture><br />
      <strong>Cursor</strong>
    </td>
    <td align="center" width="120">
      <picture>
        <source media="(prefers-color-scheme: dark)" srcset="./assets/readme/grok-dark.svg" />
        <img src="./assets/readme/grok.svg" width="40" height="40" alt="Grok" />
      </picture><br />
      <strong>Grok</strong>
    </td>
  </tr>
</table>

## Connect a custom AI provider

For local sessions, open **Settings → Agents → OpenCode → Providers** and add a
custom provider:

1. Enter a name and the provider's API base URL, including `/v1` if required by
   that provider. Proliferate uses the URL as entered.
2. Choose **Chat Completions** or **Responses** to match the provider's API.
3. Enter an API key if required. Leave it empty for a provider without
   authentication.
4. Enter the model IDs supplied by the provider, one per line, and save.

Connections can be enabled or removed individually. API keys are stored in the
encrypted credential vault. Models become selectable after the runtime discovers
them through OpenCode; saving a connection does not verify that its endpoint is
reachable or that every model supports coding tools. Existing provider logins
remain available.

This configuration currently applies to local OpenCode sessions. It adds model
choices to OpenCode; it does not create a standalone agent or configure model
access for other harnesses. Other harnesses retain their own authentication and
provider settings.

## Connect an ACP agent

For local sessions, open **Settings → Agents → Custom agents → Connect agent**.
Enter a name, the installed executable's absolute path or command on `PATH`,
arguments as a JSON array (for example, `["--acp"]`), and optional environment
values as a JSON object. The executable must speak Agent Client Protocol over
standard input and output; an ordinary chat API endpoint is configured as a
provider instead.

Proliferate discovers the agent's models and controls through ACP and presents
them in the existing agent picker. Use **Refresh models** to request discovery
again. Custom agents are installed and updated by you. Their registrations and
environment values stay on the local runtime, with environment values hidden
after saving.

An agent must advertise model choices through ACP to appear in the model picker.

Removing a registration leaves running sessions alone, but sessions using it
cannot launch again without that registration.

## Use agent skills

Open **Settings → Agents → Skills** and import a directory with a root
`SKILL.md`. Include any relative resources referenced by the instructions in
the same directory. Imports are disabled initially; enable a package for all
repositories or choose a repository override. A repository override can also
disable a globally enabled skill.

Enabled instructions are delivered when an agent session launches. Resource
files are copied into a private `.proliferate-skills` directory in the workspace
so the agent can read them. These copies remain available for session resumption
and can appear as untracked files unless ignored by the repository. Skills add
instructions and resources; they do not grant tools or permissions or execute
scripts automatically. Changes to enablement apply to subsequent launches.

## Self-hosting

The full Proliferate control plane is self-hostable. Start with the
[deployment docs](https://proliferate.com/docs/deployment), which cover Docker,
AWS, GCP, Azure, Kubernetes, and air-gapped operation.

- **Docker Compose:** [self-hosted-deploy.md](./guides/deploying/self-hosted-deploy.md)
  runs Caddy, Postgres, and the API, with bootstrap and update scripts
- **AWS (one-click):** [self-hosted-aws.md](./guides/deploying/self-hosted-aws.md)
  is a CloudFormation wrapper that provisions the stack on EC2
- **Configuration:** [`server/deploy/.env.production.example`](./server/deploy/.env.production.example)
  documents every required and optional setting

Point the desktop app at your control plane by following
[configure desktop](https://proliferate.com/docs/deployment/configure-desktop).
[Open an issue](https://github.com/proliferate-ai/proliferate/issues/new/choose) or ask in
[Discord](https://discord.gg/2RVNNzEZnj) if you hit problems, and see
[SECURITY.md](./SECURITY.md) for reporting vulnerabilities.

## Try the app from source

Requirements:

- Rust through `rustup` (the repository pins its toolchain in `rust-toolchain.toml`)
- Node.js 22+
- pnpm
- Python 3.12 and `uv` for the local control plane
- Docker, or existing local Postgres and Redis services
- On macOS, Xcode Command Line Tools for the native desktop build

For the full app, including the credential vault used by custom providers:

On a fresh checkout, create private local server settings once. This command
refuses to overwrite existing settings:

```bash
python3 - <<'PY'
import os, secrets
from pathlib import Path

settings = (
    f"JWT_SECRET={secrets.token_urlsafe(48)}\n"
    f"CLOUD_SECRET_KEY={secrets.token_urlsafe(48)}\n"
    "SINGLE_ORG_MODE=true\n"
    f"SETUP_TOKEN_FILE={Path.home()}/.proliferate-local/setup-token\n"
)
fd = os.open("server/.env.local", os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600)
with os.fdopen(fd, "w") as file:
    file.write(settings)
PY
```

Then install and launch:

```bash
make install
make server-install
SINGLE_ORG_MODE=true make dev PROFILE=tryout
```

`make dev` prepares an isolated profile, builds changed components, applies
database migrations, and starts the desktop app, API, runtime, and web client.
The first build can take several minutes. Keep that terminal running; stop the
profile with Ctrl-C. Use a different profile name for each worktree.

Inspect profiles and their allocated ports with:

```bash
make dev-list
```

On first launch, open `http://127.0.0.1:<API-port>/setup`, using the API port
shown for the profile. Read the setup token from the private file named by
`SETUP_TOKEN_FILE` in your local settings, then create your local owner account.
Sign in with that account in the app to use the credential vault. Keep the
local settings file: its encryption key protects saved credentials. See
[local account setup](./guides/local/feature-worktree-auth.md#layer-b--feature-needs-a-real-backend-session-adminorgauthed-apis)
for details.

Check install results in **Settings → Agents**. With npm 11.19.0,
managed Claude/Codex adapter preparation can fail with `EALLOWSCRIPTS` when
the user npm configuration contains `allow-scripts`; npm tracks this in
[its Git-dependency preparation issue](https://github.com/npm/cli/issues/9783).
OpenCode installs directly, and existing ACP executables can be registered
under **Custom agents**.

For the desktop and local runtime without a control plane:

```bash
make install
make dev-local
```

This mode uses the agents' own authentication. Custom provider settings require
a reachable, signed-in control plane for the encrypted vault.

See [running locally](./guides/local/README.md) for database prerequisites and
[dev profiles](./guides/local/dev-profiles.md) for profile state, ports, generated
Tauri config, and app labels. A downloaded release does not contain uncommitted
changes from this checkout; build from source to try these changes.

## Intel preview DMG from GitHub

The fork's **Build Intel preview DMG** workflow builds branch `build/intel-dmg`
on a GitHub-hosted Intel Mac. Find the run under **Actions**, then download
`proliferate-preview-intel-dmg` from its artifacts. The download contains
`Proliferate-Preview-intel.dmg`, `SHA256SUMS`, and build information identifying
the source commit. Verify the checksum before installing:

```bash
shasum -a 256 -c SHA256SUMS
```

The preview requires macOS 15 or newer. It has its own app identifier, is
ad-hoc signed without Apple notarization, and has automatic updates disabled.
macOS may require manual approval to open it. Ad-hoc signatures can also cause
Keychain permission prompts or require signing in again after a rebuild.

The default control-plane URL is `http://127.0.0.1:8000`. Keep the local API
running and sign in to use the custom-provider vault; the DMG bundles the
runtime and agent seed, not PostgreSQL, Redis, or the API server. This workflow
only uploads a test artifact; it does not publish a GitHub Release or an updater.

## Community

Join our community on [Discord](https://discord.gg/2RVNNzEZnj)!

## Contributing

Contributing? See the [Contribution Guide](./CONTRIBUTING.md).

## License

[AGPL-3.0](./LICENSE)
