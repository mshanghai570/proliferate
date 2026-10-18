import type { SessionMcpServer } from "@anyharness/sdk";

function record(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function pairs(value: unknown): { name: string; value: string }[] {
  if (value === undefined) return [];
  if (!record(value) || Object.values(value).some((item) => typeof item !== "string")) {
    throw new Error("Headers and environment variables must be JSON objects with string values.");
  }
  return Object.entries(value).map(([name, item]) => ({ name, value: item as string }));
}

/** Drafts stay in memory; error messages never include the submitted text. */
export function parseProjectMcpServer(name: string, source: string): SessionMcpServer {
  if (!/^[a-zA-Z0-9_-]{1,50}$/.test(name) || name.toLowerCase().startsWith("proliferate_")) {
    throw new Error("Use 1–50 letters, digits, hyphens, or underscores; proliferate_ is reserved.");
  }
  if (new TextEncoder().encode(source).length > 65_536) throw new Error("Use a configuration smaller than 64 KiB.");
  let value: unknown;
  try { value = JSON.parse(source); } catch { throw new Error("Enter valid JSON without comments or trailing commas."); }
  if (!record(value)) throw new Error("Enter one HTTP or stdio server object.");
  const common = { connectionId: `local-project:${name}`, serverName: name, catalogEntryId: null };
  if (value.transport === "http" && typeof value.url === "string") {
    if (Object.keys(value).some((key) => !["transport", "url", "headers"].includes(key))) {
      throw new Error("HTTP settings support transport, url, and headers.");
    }
    return { ...common, transport: "http", url: value.url, headers: pairs(value.headers) };
  }
  if (value.transport === "stdio" && typeof value.command === "string") {
    if (Object.keys(value).some((key) => !["transport", "command", "args", "env"].includes(key))) {
      throw new Error("Stdio settings support transport, command, args, and env.");
    }
    const args = value.args === undefined ? [] : value.args;
    if (!Array.isArray(args) || !args.every((arg): arg is string => typeof arg === "string")) {
      throw new Error("Arguments must be a JSON array of strings.");
    }
    return { ...common, transport: "stdio", command: value.command, args, env: pairs(value.env) };
  }
  throw new Error("Choose transport http with a url, or stdio with a command.");
}
