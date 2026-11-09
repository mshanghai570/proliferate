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

- **[Native harnesses](https://proliferate.com/docs/product/agents)** - Claude Code, Codex, OpenCode, Cursor, Grok, and more
- **Local AI providers** - connect OpenAI-compatible endpoints to local OpenCode sessions with encrypted credentials on your Mac; no Proliferate sign-in is required
- **Custom ACP agents** - connect installed agents with their own command, arguments, and environment
- **Agent skills** - import `SKILL.md` packages and resources, then enable them for all repositories or individual repositories
- **[Worktree workspaces](https://proliferate.com/docs/product/workspaces)** - an isolated branch and working directory for every task
- **[Parallel agents](https://proliferate.com/docs/concepts/parallel-agents)** - run agents side by side in the same workspace, each on its own task
- **[Subagents](https://proliferate.com/docs/features/subagents)** - agents delegate scoped work to child agents and pick the results back up when they finish
- **[Integrations](https://proliferate.com/docs/product/integrations)** - MCPs, skills, Computer Use, Browser Use, and custom tools, configured once and shared by every agent
- **Local project MCP servers** - configure HTTP and stdio servers for a repository's local sessions, with encrypted credentials and enable, disable, replace, and remove controls
- **Extensions marketplace (preview)** - browse Open VSX extensions, install supported declarative packages, and apply supported app and syntax theme colors in the macOS app; extension code remains inactive
- **[Workflows](https://proliferate.com/docs/product/workflows)** - recurring and event-driven agent runs: nightly review passes, triage on alerts, dependency bumps

Personal Intel preview builds run in GitHub Actions through
**Build Intel preview DMG**. Feature branches can be built by manually starting
that workflow with the desired branch. Download the DMG from that run's artifacts. The separate Preview app requires
macOS 15+, is ad-hoc signed without notarization. Account features connect to
`https://app.proliferate.com/api` by default. Account sign-in is optional; local
agents, API providers, and project MCP settings use the bundled runtime.

### Daily-driver release audit

The release target is a versioned, ad-hoc-signed DMG published on GitHub, with
checksums and release notes. Apple notarization is unavailable for this build.
Current qualification status:

| Area | Current state |
| --- | --- |
| Local agent login | In-app provider login and refresh available without a product account; client workflow checks pass. Live provider login still needs hands-on qualification. |
| Proliferate account | Client connection and callback checks pass; hosted service returned `503 DEPLOYMENT_PAUSED` during the audit. |
| Project RAG | Local keyword indexing, cited search, and an explicit composer picker are authored; cloud qualification is pending. |
| Per-agent rules | Private Markdown storage, repository replacements, launch delivery, and Settings → Agents → Rules are authored; cloud qualification is pending. |
| Marketplace, MCP, skills | Implemented scopes are described below; executable extension hosting is unavailable. |
| Distribution | GitHub builds produce verified DMG artifacts; no versioned daily-driver release has been published. |

### Agent rules and project context

Settings → Agents → Rules edits Markdown instructions for any runtime agent,
including custom ACP agents. Import `.md` or `.txt` files, save a user default,
or replace that default for a repository and its worktrees. An empty repository
file suppresses the default; “Use user default” removes the override. Rules
apply on the next agent process start, including resume. Existing repository
instruction files remain independent. Rules are limited to 64 KiB of UTF-8 text.

The local chat composer’s **Project context** control explicitly indexes a Git
workspace and searches source using SQLite FTS5 keyword ranking. Review results
with file and line citations, select excerpts, then add them to the existing
draft. Nothing is sent to a model until you send the prompt. Indexing uses Git
ignores, source-file limits, credential exclusions, and symlink rejection;
source may still contain private data. Search refreshes changed and deleted
files. The picker exposes partial-index limits and can clear the local cache.
This version has no vector embeddings or automatic context injection.

These additions are under cloud qualification. The
[macOS fork release procedure](guides/deploying/fork-macos.md) describes native
DMG builds and versioned GitHub publication without Apple credentials.

### Current macOS customization scope

The Extensions destination is visible by default in the macOS app. It provides
Open VSX discovery, package installation, inventory, and removal. Registry
requests use browser-managed caching without custom headers, allowing the
macOS webview to access the public registry under its CORS policy. Open **Details
and versions** from an installed package to inspect its registry listing and
version history, then return to Installed. In **Installed →
App color themes**, copy readable core colors from a declared standalone JSON or JSONC
theme into Appearance settings. The copy survives package removal and can be
reset in Appearance. **Installed → Syntax themes** applies foreground token
colors from a supported theme to source views, code blocks, and diffs, together
with matching background/text colors. The copied theme persists across restarts and package
removal. Reset it in **Appearance → Imported syntax colors**. Theme includes and executable extensions
are unsupported. Extension code does not activate in the editor. Set
`VITE_EXTENSION_MARKETPLACE=0` when building Desktop to hide the preview.

Local OpenAI-compatible providers add models to local OpenCode sessions without
a Proliferate account. They do not create standalone agents or configure every harness.
Custom ACP agents are a separate option and must implement Agent Client
Protocol. MCP tools are available through supported integrations and local
project server settings described below. Appearance
settings cover built-in color modes and text sizing. The macOS app can import
core app colors from a readable, strict JSON color theme with `type`,
`editor.background`, and `editor.foreground`. Syntax colors are available from
supported installed packages through **Syntax themes**. This accepts at most 256
token rules and 32 KiB of scope/color data per theme. Token font styles,
semantic-token overrides, includes, and external token files are unsupported.
Theme files must use six-digit hex color values without comments or trailing
commas. In the macOS app, enter
installed font family names for interface and code text under **Settings →
Appearance**. The Mac font list offers suggestions; an unavailable manually
entered family falls back to the built-in font. Project context retrieval and agent rules are described below; their cloud qualification is pending.

A minimal file for **Settings → Appearance → Imported app colors**:

```json
{
  "name": "Midnight",
  "type": "dark",
  "colors": {
    "editor.background": "#101820",
    "editor.foreground": "#FFFFFF"
  }
}
```

### macOS product direction

The following goals describe work to make the macOS app a complete coding
workspace. They are not claims about current availability.

- **Extension marketplace:** discover, install, update, enable, disable, and
  remove editor extensions. Activate supported themes, language features,
  snippets, and other contributions in the running editor, with visible
  compatibility and permissions.
- **Built-in multi-agent orchestration:** run dual-agent setups and larger
  agent swarms on one project. Choose the harness, provider, model, and
  instructions for each agent, then coordinate and review their work in the
  app. One setup should be able to pair an OpenCode model with Codex without
  requiring an agent skill to orchestrate them.
- **Provider choice per agent:** configure compatible model providers as a
  first-class capability alongside native harnesses and ACP agents.
- **Project RAG:** index project content, retrieve relevant context for agent
  tasks, and show what information is indexed and sent to a model.
- **MCP management:** extend local project server management with connection
  checks, tool inspection, OAuth setup, and access controls for individual agents.
- **Editor personalization:** extend syntax themes to font styles, semantic
  tokens, and theme includes; add font previews to the installed-family choices.

These goals can ship in stages. The feature list above should grow only when
the corresponding capability works in the macOS app.

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

Open **Settings → Agents → LLM providers → Add API provider**. These connections
are saved by the local runtime; no control plane or Proliferate account is needed.
Custom ACP agents have a separate settings page.

1. Choose **Custom endpoint**, **Ollama**, or **LM Studio**. Local presets fill
   the default server address; start that server and load a model first.
   Switching setup clears the key and model fields.
2. Enter a name and the provider's API base URL, including its API base path,
   such as `/v1`. Proliferate uses the URL as entered.
3. Choose **Chat Completions** or **Responses** to match the provider's API.
4. Enter an API key if required. Leave it empty for a keyless server.
5. Enter model IDs, one per line, and save.

Connections can be enabled, disabled, and removed independently. Credentials
are encrypted with the runtime's data key, which the desktop app keeps in the
Mac keychain. Inventory responses never return keys or full configurations.
To change a saved connection, remove it and add it again. The runtime supports
up to 32 local connections and 100 model IDs per connection.

Choose **OpenCode** when creating a chat, then select one of the runtime's
reported models. Use **Models** on the provider page to inspect and refresh the
list. Saving does not test connectivity or confirm coding tool support. Changes
apply to new agent processes; running sessions retain their credentials.

When a managed authentication profile is applied, local connections are paused.
The app shows that state and permits disabling or removing local entries.
**Runtime setup and managed provider settings** opens OpenCode's existing
configuration page, including hosted-provider keys in the control-plane vault.
The local vault is separate and is never uploaded by the auth-state courier.

The local presets use documented compatibility endpoints for
[Ollama](https://github.com/ollama/ollama/blob/main/docs/api/openai-compatibility.mdx)
and [LM Studio](https://lmstudio.ai/docs/developer/openai-compat).

This configuration applies to local OpenCode sessions. Other harnesses retain
their own authentication and provider settings.

## Connect a local project MCP server

In the macOS app, open **Settings → Repo**, select your local repository,
and open **Environment → Project MCP servers**. Enter a server name and its JSON
configuration, then choose **Save server**. For example:

```json
{
  "transport": "http",
  "url": "http://localhost:8000/mcp",
  "headers": {}
}
```

For a local process, use `"transport": "stdio"`, a `command`, optional `args`
(an array of strings), and optional `env` (an object with string values). A
command can be an absolute executable path or a command available on the app's
shell `PATH`; arguments are passed separately, without shell expansion.

```json
{
  "transport": "stdio",
  "command": "/absolute/path/to/mcp-server",
  "args": [],
  "env": {}
}
```

Configurations and credentials are encrypted in the local runtime database
using the macOS app's keychain-backed data key. Credentials cannot be read back
from the settings screen. To edit a server, enter the same name and its complete
configuration, then confirm replacement. Saving a replacement enables it.
Each project supports up to 32 servers; names have 1–50 letters, digits, hyphens,
or underscores and cannot start with `proliferate_`.

Enabled servers are passed to supported agent harnesses when a session process
starts, including sessions in that repository's worktrees. Disable or remove a
server to exclude it from future launches. Running sessions keep their current
tools until their process stops. Start a new session to use saved changes
immediately. Saving does not check connectivity or discover
tools. Harness support determines which transports and tools are usable. OAuth
setup, tool inspection, and per-agent server selection are not yet available.
Project MCP settings remain local and are not exported with workspace archives.

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

## Sign in to local agents

Local ACP agents do not require a Proliferate account. Open **Settings → Agents**,
choose the agent, then use **Agent sign-in → Authenticate** to open its supported
login flow on this Mac. Complete the provider's login in the terminal/browser;
**Refresh agent sign-in** checks credentials already saved by the provider CLI.
Agents without an in-app login command use their documented CLI login. For
OpenCode API connections, use **Settings → Agents → LLM providers**.

Proliferate account sign-in is separate: it needs a reachable, configured control
plane. The personal preview targets Proliferate's hosted service at
`https://app.proliferate.com/api`. Open **Settings → Account** for the sign-in
methods supported by that service. If the service is unavailable, retry the
account connection from the same screen; local agent login, API providers,
skills, and project MCP settings remain available.

**Account server → Connect to a server** opens server validation and confirmation
before changing the saved endpoint and restarting the app. Existing local or
self-hosted overrides can choose **Use Proliferate hosted service**. Overrides in
`~/.proliferate/config.json` take precedence over the packaged default. The
preview registers `proliferate-local://auth/callback` for browser login, separate
from the production app's `proliferate` scheme. The service must accept this
callback and be online for hosted sign-in to complete.

## Use agent skills

Open **Settings → Agents → Skills**. To import one instructions file, enter a
skill name and choose **Import SKILL.md file**; lowercase `skill.md` is also
accepted and stored as `SKILL.md`. Instructions must contain 1–65,536 bytes of
UTF-8 text. Use **Import skill folder** for a directory with a root `SKILL.md`
and any relative resources referenced by the instructions. Imports are disabled initially; enable a package for all
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
