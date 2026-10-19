# Fork macOS DMG release

This procedure is limited to `mshanghai570/proliferate`. It uses GitHub runners,
not local compiler output, and publishes an Intel macOS 15+ application with an
ad-hoc signature. Apple signing credentials and notarization are unavailable.
The native app retains `com.mshanghai570.proliferate.preview` and the displayed
name Proliferate Preview so updates preserve its existing identity and settings.

## Build and qualify

Dispatch `build-intel-dmg.yml` at the exact fork branch with `app_version` set
to a numeric `major.minor.patch` value. The workflow builds the desktop frontend,
checks focused account/provider, marketplace, skills, rules and retrieval flows,
compiles the native runtime, regenerates API types, verifies compatibility, runs
native tests, and builds the native app and DMG. It verifies architecture,
ad-hoc signatures, callback scheme registration, version, and mounted contents.

Wait for the whole run to succeed. The `proliferate-preview-intel-dmg` artifact
contains the DMG, `SHA256SUMS`, and `BUILD-INFO.txt` with the exact commit/version.
A generated-contract artifact is for source synchronization, not installation.
Keep generated source synchronized before building consumers of new API types.

Before describing a build as qualified for your use, install the artifact and
complete login with your provider, send a prompt, reopen the app, and confirm
project settings persist. Hosted Proliferate account login requires its service
to be available. A paused hosted deployment cannot be repaired by packaging.

## Publish on GitHub

Use the existing build workflow entry point on a branch containing the release
workflow changes. Set `publish_build_run_id` to the fully successful build run
and `release_tag` to an unused `macos-vMAJOR.MINOR.PATCH` matching its app version.
Leave `publish_build_run_id` empty for ordinary builds.

The publish-only dispatch skips compilation and calls `publish-fork-macos.yml`.
It rejects failed or unfinished runs, other workflows/repositories, non-fork
branches, expired or ambiguous artifacts, reused tags, source/version mismatches,
and invalid checksums. Only the publish job has contents-write permission.
It creates a GitHub prerelease targeting the exact built commit, with the DMG,
checksum, build details, platform requirements, and current capability limits.
It does not enable the production updater or change upstream release delivery.

If publication stops after creating a tag, inspect that release before taking
any further action; the workflow refuses to overwrite the tag or its assets.

## Install

Download the DMG and checksum files from the release. In their directory run
`shasum -a 256 -c SHA256SUMS`. Open the DMG and drag the app into Applications.
Because the app is not notarized, macOS may require approval in System Settings
→ Privacy & Security after the first launch attempt. Launch from Applications;
complete provider authentication in Settings → Agents. Local providers and MCP
settings do not require a Proliferate account.

See [release delivery](../../specs/engineering/ci-cd/release-delivery.md) for the
canonical upstream release system; this fork workflow is an isolated channel.
