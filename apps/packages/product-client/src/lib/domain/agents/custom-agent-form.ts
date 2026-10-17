import type { RegisterCustomAcpAgentRequest } from "@anyharness/sdk";

export function parseCustomAgentForm(name: string, command: string, argsText: string, envText: string): RegisterCustomAcpAgentRequest {
  let args: unknown;
  let env: unknown;
  try {
    args = JSON.parse(argsText.trim() || "[]");
    env = JSON.parse(envText.trim() || "{}");
  } catch {
    throw new Error("Arguments and environment must be valid JSON.");
  }
  if (!Array.isArray(args) || !args.every((value) => typeof value === "string")) {
    throw new Error("Arguments must be a JSON array of strings.");
  }
  if (env === null || Array.isArray(env) || typeof env !== "object"
    || !Object.values(env).every((value) => typeof value === "string")) {
    throw new Error("Environment must be a JSON object with string values.");
  }
  return { name: name.trim(), command: command.trim(), args, env: env as Record<string, string> };
}
