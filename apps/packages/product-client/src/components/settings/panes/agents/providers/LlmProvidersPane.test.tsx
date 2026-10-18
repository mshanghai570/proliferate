// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { MemoryRouter } from "react-router-dom";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import { LlmProvidersPane } from "#product/components/settings/panes/agents/providers/LlmProvidersPane";

const mocks = vi.hoisted(() => ({ editor: vi.fn(), providers: vi.fn(), models: vi.fn() }));
vi.mock("#product/hooks/agents/workflows/use-harness-auth-editor", () => ({ useHarnessAuthEditor: mocks.editor }));
vi.mock("#product/components/settings/panes/agents/harness/HarnessProvidersSection", () => ({
  HarnessProvidersSection: (props: unknown) => { mocks.providers(props); return <div>Provider controls</div>; },
}));
vi.mock("#product/components/settings/panes/agents/harness/HarnessAllModelsSection", () => ({
  HarnessAllModelsSection: (props: unknown) => { mocks.models(props); return <div>Runtime models</div>; },
}));
afterEach(cleanup);
beforeEach(() => vi.clearAllMocks());
function editor(overrides = {}) {
  return { apiKeysQuery: { data: [], refetch: vi.fn() }, selectionsQuery: { data: [], refetch: vi.fn() }, deliveryPending: false, ...overrides };
}
function mount() { render(<MemoryRouter><LlmProvidersPane /></MemoryRouter>); }

it("manages local provider bindings and displays runtime-reported models", () => {
  const state = editor({ deliveryPending: true });
  mocks.editor.mockReturnValue(state);
  mount();
  expect(screen.getByRole("heading", { name: "LLM providers" })).toBeTruthy();
  expect(mocks.editor).toHaveBeenCalledWith("opencode", "OpenCode", "local");
  expect(mocks.providers).toHaveBeenCalledWith({ editor: state });
  expect(mocks.models).toHaveBeenCalledWith({ harnessKind: "opencode", displayName: "OpenCode", surface: "local" });
  expect(screen.getByText("Applying provider settings to this Mac…")).toBeTruthy();
  expect(screen.getByRole("link", { name: "Set up the OpenCode runtime" }).getAttribute("href")).toBe("/settings?section=agent-opencode");
  expect(screen.getByText(/Other harnesses keep their own provider settings/)).toBeTruthy();
});
it("waits for saved settings before permitting edits", () => {
  mocks.editor.mockReturnValue(editor({ selectionsQuery: { data: undefined } }));
  mount();
  expect(screen.getByText("Loading saved provider settings…")).toBeTruthy();
  expect(mocks.providers).not.toHaveBeenCalled();
});
it("offers a retry and prevents writes after a configuration read fails", () => {
  const keys = { data: [], isError: true, refetch: vi.fn() };
  const selections = { data: [], refetch: vi.fn() };
  mocks.editor.mockReturnValue(editor({ apiKeysQuery: keys, selectionsQuery: selections }));
  mount();
  fireEvent.click(screen.getByRole("button", { name: "Retry provider settings" }));
  expect(keys.refetch).toHaveBeenCalledOnce();
  expect(selections.refetch).toHaveBeenCalledOnce();
  expect(mocks.providers).not.toHaveBeenCalled();
});
