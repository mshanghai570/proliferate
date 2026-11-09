// @vitest-environment jsdom
import type { ReactNode } from "react";
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { MemoryRouter } from "react-router-dom";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import { LlmProvidersPane } from "#product/components/settings/panes/agents/providers/LlmProvidersPane";

const mocks = vi.hoisted(() => ({ editor: vi.fn(), models: vi.fn(), scope: "runtime-a" }));
vi.mock("#product/hooks/access/anyharness/use-local-providers", () => ({ useLocalProviders: () => ({ identity: mocks.scope }) }));
vi.mock("#product/hooks/agents/workflows/use-local-provider-editor", () => ({ useLocalProviderEditor: mocks.editor }));
vi.mock("#product/components/settings/panes/agents/harness/HarnessAllModelsSection", () => ({ HarnessAllModelsSection: (props: unknown) => { mocks.models(props); return <div>Runtime models</div>; } }));
vi.mock("#product/primitives/patterns/ModalShell", () => ({ ModalShell: ({ children }: { children: ReactNode }) => <div>{children}</div> }));
vi.mock("#product/primitives/patterns/ConfirmationDialog", () => ({ ConfirmationDialog: () => null }));
afterEach(cleanup);
beforeEach(() => { vi.clearAllMocks(); mocks.scope = "runtime-a"; });
function editor(overrides = {}) {
  return { connected: true, pending: false, open: false, removeId: null, error: null, notice: null,
    query: { data: { providers: [], encryptionAvailable: true, managedAuthActive: false }, refetch: vi.fn() },
    show: vi.fn(), close: vi.fn(), toggle: vi.fn(), requestRemove: vi.fn(), submit: vi.fn(), confirmRemove: vi.fn(), ...overrides };
}
function pane() { return <MemoryRouter><LlmProvidersPane /></MemoryRouter>; }

it("offers local provider setup without a cloud auth editor", () => {
  const state = editor(); mocks.editor.mockReturnValue(state); render(pane());
  expect(screen.getByRole("heading", { name: "LLM providers" })).toBeTruthy();
  expect(screen.getByText(/No Proliferate sign-in is required/)).toBeTruthy();
  fireEvent.click(screen.getByRole("button", { name: "Add API provider" }));
  expect(state.show).toHaveBeenCalledOnce();
  expect(mocks.models).toHaveBeenCalledWith({ harnessKind: "opencode", displayName: "OpenCode", surface: "local" });
});
it("blocks editing while saved providers load or the runtime is disconnected", () => {
  mocks.editor.mockReturnValue(editor({ query: { isPending: true } })); render(pane());
  expect(screen.getByText("Loading local providers…")).toBeTruthy();
  expect((screen.getByRole("button", { name: "Add API provider" }) as HTMLButtonElement).disabled).toBe(true);
});
it("permits disabling and removing local connections while managed auth pauses them", () => {
  const state = editor({ query: { data: { providers: [{ id: "one", name: "My models", modelCount: 1, enabled: true }], encryptionAvailable: true, managedAuthActive: true } } });
  mocks.editor.mockReturnValue(state); render(pane());
  expect(screen.getByText(/Local connections are paused/)).toBeTruthy();
  expect((screen.getByRole("button", { name: "Add API provider" }) as HTMLButtonElement).disabled).toBe(true);
  fireEvent.click(screen.getByRole("button", { name: "Disable My models" }));
  fireEvent.click(screen.getByRole("button", { name: "Remove My models" }));
  expect(state.toggle).toHaveBeenCalledWith("one", false);
  expect(state.requestRemove).toHaveBeenCalledWith("one");
});
it("offers retry when an older or disconnected runtime cannot read the inventory", () => {
  const query = { isError: true, refetch: vi.fn() }; mocks.editor.mockReturnValue(editor({ query })); render(pane());
  fireEvent.click(screen.getByRole("button", { name: "Retry provider settings" }));
  expect(query.refetch).toHaveBeenCalledOnce();
  expect((screen.getByRole("button", { name: "Add API provider" }) as HTMLButtonElement).disabled).toBe(true);
});
it("clears credential drafts when runtime or actor identity changes", () => {
  mocks.editor.mockReturnValue(editor({ open: true }));
  const mounted = render(pane());
  fireEvent.change(screen.getByLabelText("API key (optional)"), { target: { value: "secret" } });
  mocks.scope = "runtime-b"; mounted.rerender(pane());
  expect((screen.getByLabelText("API key (optional)") as HTMLInputElement).value).toBe("");
});
