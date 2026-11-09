import { describe, expect, it } from "vitest";
import { parseCustomAgentForm } from "./custom-agent-form";

describe("custom ACP registration form", () => {
  it("preserves literal arguments and environment values without shell expansion", () => {
    expect(parseCustomAgentForm(" My agent ", " /opt/agent ", '["--acp", "two words", "", "$HOME"]', '{"TOKEN":" secret "}'))
      .toEqual({ name: "My agent", command: "/opt/agent", args: ["--acp", "two words", "", "$HOME"], env: { TOKEN: " secret " } });
  });
  it("allows omitted arguments and environment", () => {
    expect(parseCustomAgentForm("Agent", "agent", "", "")).toEqual({ name: "Agent", command: "agent", args: [], env: {} });
  });
  it.each([['"--acp"', "{}"], ["[1]", "{}"], ["[]", "null"], ["[]", "[]"], ["[]", '{"PORT":123}']])("rejects non-string configuration shapes", (args, env) => {
    expect(() => parseCustomAgentForm("Agent", "agent", args, env)).toThrow();
  });
  it("does not expose invalid environment content in parser errors", () => {
    expect(() => parseCustomAgentForm("Agent", "agent", "[]", "private-secret")).toThrow("Arguments and environment must be valid JSON.");
  });
});
