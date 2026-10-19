// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, expect, it, vi } from "vitest";
import type { HarnessAuthEditorApi } from "#product/hooks/agents/workflows/use-harness-auth-editor";
import { LocalHarnessAuthentication } from "./LocalHarnessAuthentication";

vi.mock("#product/components/agents/AgentLoginTerminalPanel", () => ({
  AgentLoginTerminalPanel: ({ baseUrl, onClose, onRestart }: { baseUrl: string; onClose: (kind: string) => void; onRestart: () => void }) => (
    <div>Login terminal: {baseUrl}<button onClick={() => onClose("codex")}>Close login</button><button onClick={onRestart}>Restart login</button></div>
  ),
}));
afterEach(cleanup);
function editor(overrides = {}) {
  return {
    authReady: false, surface: "local",
    localAgent: { kind: "codex", displayName: "Codex", readiness: "login_required", cliAuthState: "absent", supportsLogin: true },
    loginSession: undefined,
    loginWorkflow: { openAuthTerminal: vi.fn(), closeAuthTerminal: vi.fn(), handleTerminalExit: vi.fn(),
      refreshAgentReadiness: vi.fn(async () => {}), runtimeConnection: { baseUrl: "http://local-runtime", authToken: "runtime-token" } },
    ...overrides,
  } as unknown as HarnessAuthEditorApi;
}
it("opens provider login without a product account", () => {
  const value = editor(); render(<LocalHarnessAuthentication displayName="Codex" editor={value} />);
  expect(screen.getByText(/A Proliferate account is not required/)).toBeTruthy();
  fireEvent.click(screen.getByRole("button", { name: "Authenticate" }));
  expect(value.loginWorkflow.openAuthTerminal).toHaveBeenCalledWith(value.localAgent, { restart: false });
});
it("refreshes CLI credentials and reports connection failures", async () => {
  const value = editor(); vi.mocked(value.loginWorkflow.refreshAgentReadiness).mockRejectedValue(new Error("offline"));
  render(<LocalHarnessAuthentication displayName="Codex" editor={value} />);
  fireEvent.click(screen.getByRole("button", { name: "Refresh agent sign-in" }));
  await waitFor(() => expect(screen.getByRole("alert").textContent).toContain("Could not refresh agent sign-in"));
  expect(value.loginWorkflow.refreshAgentReadiness).toHaveBeenCalledOnce();
});
it("keeps local terminal close and restart controls available", () => {
  const value = editor({ loginSession: { kind: "codex", isStarting: false, terminal: { id: "terminal-1" }, errorMessage: null } });
  render(<LocalHarnessAuthentication displayName="Codex" editor={value} />);
  expect(screen.getByText(/Login terminal: http:\/\/local-runtime/)).toBeTruthy();
  fireEvent.click(screen.getByRole("button", { name: "Close login" }));
  expect(value.loginWorkflow.closeAuthTerminal).toHaveBeenCalledWith("codex");
  fireEvent.click(screen.getByRole("button", { name: "Restart login" }));
  expect(value.loginWorkflow.openAuthTerminal).toHaveBeenCalledWith(value.localAgent, { restart: true });
});
it("does not offer login when the CLI is already authenticated", () => {
  render(<LocalHarnessAuthentication displayName="Codex" editor={editor({ localAgent: {
    kind: "codex", readiness: "ready", cliAuthState: "authenticated", supportsLogin: true,
  } })} />);
  expect(screen.getByText("Authenticated")).toBeTruthy();
  expect(screen.queryByRole("button", { name: "Authenticate" })).toBeNull();
});
it("provides instructions for agents without a login command", () => {
  render(<LocalHarnessAuthentication displayName="OpenCode" editor={editor({ localAgent: {
    kind: "opencode", readiness: "missing_env", supportsLogin: false,
  } })} />);
  expect(screen.getByText(/Use the agent's documented login/)).toBeTruthy();
  expect(screen.queryByRole("button", { name: "Authenticate" })).toBeNull();
});
