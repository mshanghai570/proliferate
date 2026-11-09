import {
  useCreateAgentProviderConfig,
  useRevokeAgentApiKey,
} from "@proliferate/cloud-sdk-react";

export function useCustomProviderAccess() {
  return {
    create: useCreateAgentProviderConfig(),
    revoke: useRevokeAgentApiKey(),
  };
}
