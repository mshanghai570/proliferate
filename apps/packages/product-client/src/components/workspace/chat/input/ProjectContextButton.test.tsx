// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen, within } from "@testing-library/react";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import { ProjectContextButton } from "./ProjectContextButton";
const state = vi.hoisted(() => ({ refresh: vi.fn(), clear: vi.fn(), search: vi.fn(), scope: "actor", local: true,
  enabled: false, queryError: false, results: false, large: false,
}));
vi.mock("@anyharness/sdk-react", () => ({ useAnyHarnessRuntimeContext: () => ({ runtimeUrl: "http://runtime" }), resolveRuntimeCacheScopeKey: () => state.scope }));
vi.mock("#product/stores/ui/agent-surface-store", () => ({ useAgentSurfaceStore: (select: (value: { surface: string }) => unknown) => select({ surface: state.local ? "local" : "cloud" }) }));
vi.mock("#product/primitives/patterns/ModalShell", () => ({ ModalShell: ({ children, footer }: { children: import("react").ReactNode; footer?: import("react").ReactNode }) => <div role="dialog">{children}<footer data-testid="context-footer">{footer}</footer></div> }));
vi.mock("#product/hooks/access/retrieval/use-project-retrieval-access", () => ({ useProjectRetrievalAccess: () => ({ available: true,
  query: { data: { enabled: state.enabled, indexedFiles: 1, chunks: 1, skippedFiles: 0 }, isSuccess: !state.queryError, isError: state.queryError, refetch: vi.fn() },
  refresh: { mutate: state.refresh, reset: vi.fn() }, clear: { mutate: state.clear, reset: vi.fn() },
  search: { mutate: state.search, reset: vi.fn(), isSuccess: state.results,
    data: state.results ? { workspaceId: "workspace", excerpts: state.large ? [0, 1, 2, 3].map((index) => ({ path: `auth${index}.ts`, startLine: 1, endLine: 1, contentHash: "a".repeat(64), text: "x".repeat(8192) })) : [{ path: "auth.ts", startLine: 1, endLine: 2, contentHash: "a".repeat(64), text: "function login() {}" }] } : undefined },
}) }));
afterEach(cleanup);
beforeEach(() => { vi.clearAllMocks(); state.scope = "actor"; state.enabled = false; state.queryError = false; state.results = false; state.local = true; state.large = false; });
it("only indexes after an explicit action", () => {
  render(<ProjectContextButton workspaceId="workspace" disabled={false} onAdd={vi.fn()} />);
  fireEvent.click(screen.getByRole("button", { name: "Project context" }));
  expect(state.refresh).not.toHaveBeenCalled();
  expect((screen.getByRole("button", { name: "Search" }) as HTMLButtonElement).disabled).toBe(true);
  fireEvent.click(screen.getByRole("button", { name: "Index project" }));
  expect(state.refresh).toHaveBeenCalledOnce();
});
it("adds only reviewed selected excerpts to the draft", () => {
  state.enabled = true; state.results = true;
  const add = vi.fn();
  render(<ProjectContextButton workspaceId="workspace" disabled={false} onAdd={add} />);
  fireEvent.click(screen.getByRole("button", { name: "Project context" }));
  expect(add).not.toHaveBeenCalled();
  expect((screen.getByRole("button", { name: "Add context to draft" }) as HTMLButtonElement).disabled).toBe(true);
  fireEvent.click(screen.getByRole("checkbox", { name: "auth.ts:1-2" }));
  fireEvent.click(screen.getByRole("button", { name: "Add context to draft" }));
  expect(add).toHaveBeenCalledWith(expect.stringContaining('Source "auth.ts":1-2'));
  expect(screen.queryByRole("dialog")).toBeNull();
});
it("drops an open picker on actor or workspace changes", () => {
  const view = render(<ProjectContextButton workspaceId="workspace" disabled={false} onAdd={vi.fn()} />);
  fireEvent.click(screen.getByRole("button", { name: "Project context" }));
  state.scope = "different actor";
  view.rerender(<ProjectContextButton workspaceId="workspace" disabled={false} onAdd={vi.fn()} />);
  expect(screen.queryByRole("dialog")).toBeNull();
  fireEvent.click(screen.getByRole("button", { name: "Project context" }));
  view.rerender(<ProjectContextButton workspaceId="other-workspace" disabled={false} onAdd={vi.fn()} />);
  expect(screen.queryByRole("dialog")).toBeNull();
});
it("does not offer local retrieval on the cloud surface", () => {
  state.local = false;
  render(<ProjectContextButton workspaceId="workspace" disabled={false} onAdd={vi.fn()} />);
  expect(screen.queryByRole("button")).toBeNull();
});

it("allows clearing an unreadable cache so it can be rebuilt", () => {
  state.queryError = true;
  render(<ProjectContextButton workspaceId="workspace" disabled={false} onAdd={vi.fn()} />);
  fireEvent.click(screen.getByRole("button", { name: "Project context" }));
  fireEvent.click(screen.getByRole("button", { name: "Clear local index" }));
  expect(state.clear).toHaveBeenCalledOnce();
});

it("searches with Enter without adding text or sending a prompt", () => {
  state.enabled = true;
  const add = vi.fn();
  render(<ProjectContextButton workspaceId="workspace" disabled={false} onAdd={add} />);
  fireEvent.click(screen.getByRole("button", { name: "Project context" }));
  const query = screen.getByRole("textbox", { name: "Search source" });
  fireEvent.change(query, { target: { value: "session auth" } });
  fireEvent.keyDown(query, { key: "Enter" });
  expect(state.search).toHaveBeenCalledWith("session auth");
  expect(add).not.toHaveBeenCalled();
});

it("shows size feedback beside the action and recovers after selecting fewer excerpts", () => {
  state.enabled = true; state.results = true; state.large = true;
  const add = vi.fn();
  render(<ProjectContextButton workspaceId="workspace" disabled={false} onAdd={add} />);
  fireEvent.click(screen.getByRole("button", { name: "Project context" }));
  screen.getAllByRole("checkbox").forEach((box) => fireEvent.click(box));
  fireEvent.click(screen.getByRole("button", { name: "Add context to draft" }));
  expect(within(screen.getByTestId("context-footer")).getByRole("alert").textContent).toContain("Choose fewer");
  expect(add).not.toHaveBeenCalled();
  fireEvent.click(screen.getAllByRole("checkbox")[3]!);
  expect(screen.queryByRole("alert")).toBeNull();
  fireEvent.click(screen.getByRole("button", { name: "Add context to draft" }));
  expect(add).toHaveBeenCalledOnce();
  expect(add.mock.calls[0]?.[0]).not.toContain("auth3.ts");
});
