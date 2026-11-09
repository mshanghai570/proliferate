import { KeyRound } from "#product/primitives/icons/core";
import { SettingsEmptyState } from "#product/primitives/patterns/settings/SettingsEmptyState";
import { PageHeader } from "#product/primitives/patterns/PageHeader";
import { SettingsPageBody } from "#product/primitives/patterns/settings/SettingsPageBody";
import { Button } from "#product/primitives/Button";
import { useCloudRepoEnvironmentEditor } from "#product/hooks/settings/workflows/use-cloud-repo-environment-editor";
import { type RepoSettingsContext } from "#product/lib/domain/settings/repo-scope-selection";
import { type SettingsRepositoryEntry } from "#product/lib/domain/settings/repositories";
import { useProductHost } from "@proliferate/product-client/host/ProductHostProvider";
import { RepoCloudGate } from "#product/components/settings/panes/repo/RepoCloudGate";
import { ProjectMcpSettings } from "#product/components/settings/panes/repo/ProjectMcpSettings";
import {
  RepoScopeEmptyState,
  type RepoScopePaneProps,
} from "#product/components/settings/panes/repo/RepoScopeStates";

/**
 * Repository environment settings: cloud variables/files and macOS project
 * MCP launch configuration. Local workspaces inherit their shell environment;
 * both editors keep saved secret values write-only.
 */
export function RepoEnvironmentPane({
  repository,
  context,
  controlPlaneReachable,
  cloudActive,
  cloudSignInChecking,
  cloudSignInAvailable,
  onSelectRepo,
  onSelectCloudEnvironment,
  onSelectRepoContext,
}: RepoScopePaneProps & {
  onSelectRepoContext: (context: RepoSettingsContext) => void;
}) {
  const host = useProductHost();
  if (!repository) {
    return (
      <RepoScopeEmptyState
        onSelectRepo={onSelectRepo}
        onSelectCloudEnvironment={onSelectCloudEnvironment}
      />
    );
  }
  return (
    <SettingsPageBody>
      <PageHeader
        variant="flat"
        title="Environment"
        description="Environment and tools for this repo's workspaces."
      />
      {context === "cloud" ? (
        <EnvironmentCloud
          repository={repository}
          controlPlaneReachable={controlPlaneReachable}
          cloudActive={cloudActive}
          cloudSignInChecking={cloudSignInChecking}
          cloudSignInAvailable={cloudSignInAvailable}
        />
      ) : (
        <>
          <EnvironmentLocal onSelectRepoContext={onSelectRepoContext} />
          {host.desktop && repository.localWorkspaceId && <ProjectMcpSettings repoRootId={repository.repoRootId} />}
        </>
      )}
    </SettingsPageBody>
  );
}

function EnvironmentCloud({
  repository,
  controlPlaneReachable,
  cloudActive,
  cloudSignInChecking,
  cloudSignInAvailable,
}: {
  repository: SettingsRepositoryEntry;
  controlPlaneReachable: boolean;
  cloudActive: boolean;
  cloudSignInChecking: boolean;
  cloudSignInAvailable: boolean;
}) {
  const editor = useCloudRepoEnvironmentEditor({ repository, cloudActive });

  return (
    <RepoCloudGate
      editor={editor}
      controlPlaneReachable={controlPlaneReachable}
      cloudActive={cloudActive}
      cloudSignInChecking={cloudSignInChecking}
      cloudSignInAvailable={cloudSignInAvailable}
    >
      {null}
    </RepoCloudGate>
  );
}

function EnvironmentLocal({
  onSelectRepoContext,
}: {
  onSelectRepoContext: (context: RepoSettingsContext) => void;
}) {
  // The cross-navigation to the cloud context is culled from desktop (ADR
  // Q3/FR-2): only web reaches the cloud repo environment, so desktop shows the
  // explanatory state without a dead "View Cloud environment" affordance.
  const host = useProductHost();
  const cloudContextReachable = host.surface === "web";
  return (
    <SettingsEmptyState
      icon={<KeyRound aria-hidden="true" />}
      title="Local environment"
      description="Local workspaces inherit variables from your shell and checkout."
      action={
        cloudContextReachable ? (
          <Button
            type="button"
            variant="secondary"
            onClick={() => onSelectRepoContext("cloud")}
          >
            View Cloud environment
          </Button>
        ) : undefined
      }
    />
  );
}
