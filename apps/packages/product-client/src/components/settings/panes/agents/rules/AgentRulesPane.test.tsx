// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import { AgentRulesPane } from "./AgentRulesPane";
const state = vi.hoisted(() => ({ save: vi.fn(), scope: "actor-a", url: "http://runtime", text: "Default instructions", error: false }));
vi.mock("@anyharness/sdk-react", () => ({
  useAnyHarnessRuntimeContext: () => ({ runtimeUrl: state.url }),
  resolveRuntimeCacheScopeKey: () => state.scope,
  useAgentsQuery: () => ({ data: [{ kind: "codex", displayName: "Codex" }, { kind: "custom-agent", displayName: "My reviewer" }] }),
  useRepoRootsQuery: () => ({ data: [{ id: "repo", path: "/project" }] }),
}));
vi.mock("#product/stores/ui/agent-surface-store", () => ({
  useAgentSurfaceStore: (select: (state: { surface: string }) => unknown) => select({ surface: "local" }),
}));
vi.mock("#product/hooks/access/agent-rules/use-agent-rules-access", () => ({
  useAgentRulesAccess: () => ({ available: true, query: { data: { text: state.text, inherited: true }, isSuccess: true, isError: state.error, refetch: vi.fn() },
    mutation: { mutateAsync: state.save, isPending: false } }),
}));
afterEach(cleanup);
beforeEach(() => { vi.clearAllMocks(); state.scope = "actor-a"; state.text = "Default instructions"; state.error = false; state.save.mockResolvedValue({}); });
it("keeps edits during refetch and locks scope until saved or discarded", async () => {
  const view = render(<AgentRulesPane />);
  const editor = screen.getByRole("textbox", { name: "Markdown instructions" });
  fireEvent.change(editor, { target: { value: "My rules" } });
  state.text = "New remote text"; view.rerender(<AgentRulesPane />);
  expect((editor as HTMLTextAreaElement).value).toBe("My rules");
  expect((screen.getByRole("combobox", { name: "Agent" }) as HTMLSelectElement).disabled).toBe(true);
  fireEvent.click(screen.getByRole("button", { name: "Save rules" }));
  await waitFor(() => expect(state.save).toHaveBeenCalledWith("My rules"));
  await waitFor(() => expect((screen.getByRole("combobox", { name: "Agent" }) as HTMLSelectElement).disabled).toBe(false));
});
it("preserves edits after failed save and isolates drafts across actors", async () => {
  state.save.mockRejectedValue(new Error("offline"));
  const view = render(<AgentRulesPane />);
  fireEvent.change(screen.getByRole("textbox"), { target: { value: "Private draft" } });
  fireEvent.click(screen.getByRole("button", { name: "Save rules" }));
  await waitFor(() => expect(screen.getByRole("alert").textContent).toContain("Could not save"));
  expect((screen.getByRole("textbox") as HTMLTextAreaElement).value).toBe("Private draft");
  state.scope = "actor-b"; view.rerender(<AgentRulesPane />);
  expect((screen.getByRole("textbox") as HTMLTextAreaElement).value).toBe("Default instructions");
});
it("allows empty overrides and custom agent selection", async () => {
  render(<AgentRulesPane />);
  expect(screen.getByRole("option", { name: "My reviewer" })).toBeTruthy();
  fireEvent.change(screen.getByRole("combobox", { name: "Agent" }), { target: { value: "custom-agent" } });
  fireEvent.change(screen.getByRole("combobox", { name: "Scope" }), { target: { value: "repo" } });
  fireEvent.change(screen.getByRole("textbox"), { target: { value: "" } });
  fireEvent.click(screen.getByRole("button", { name: "Save rules" }));
  await waitFor(() => expect(state.save).toHaveBeenCalledWith(""));
});
