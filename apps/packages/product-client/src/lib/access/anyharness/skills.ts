import type { ImportSkillRequest, SetSkillEnabledRequest } from "@anyharness/sdk";
import { getAnyHarnessClient, type AnyHarnessClientConnection } from "@anyharness/sdk-react";

export function listSkills(connection: AnyHarnessClientConnection, signal?: AbortSignal) {
  return getAnyHarnessClient(connection).skills.list({ signal });
}
export function importSkill(connection: AnyHarnessClientConnection, request: ImportSkillRequest) {
  return getAnyHarnessClient(connection).skills.import(request);
}
export function setSkillEnabled(connection: AnyHarnessClientConnection, id: string, request: SetSkillEnabledRequest) {
  return getAnyHarnessClient(connection).skills.setEnabled(id, request);
}
export function removeSkill(connection: AnyHarnessClientConnection, id: string) {
  return getAnyHarnessClient(connection).skills.remove(id);
}
