// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import { SkillsSection } from "./SkillsSection";

const state = vi.hoisted(() => ({ import: vi.fn(), available: true }));
vi.mock("@anyharness/sdk-react", () => ({
  useAnyHarnessRuntimeContext: () => ({ runtimeUrl: "http://runtime" }),
  resolveRuntimeCacheScopeKey: () => "actor-a",
  useRepoRootsQuery: () => ({ data: [] }),
}));
vi.mock("#product/stores/ui/agent-surface-store", () => ({
  useAgentSurfaceStore: (select: (value: { surface: string }) => unknown) => select({ surface: "local" }),
}));
vi.mock("#product/hooks/access/skills/use-skills-access", () => ({
  useSkillsAccess: () => ({ available: state.available, query: { data: { skills: [] }, isSuccess: true },
    mutation: { mutateAsync: state.import, isPending: false } }),
}));
afterEach(cleanup);
beforeEach(() => { vi.clearAllMocks(); state.available = true; state.import.mockResolvedValue({}); });
function instructions(name = "skill.md") {
  const file = new File(["# Review"], name, { type: "text/markdown" });
  Object.defineProperty(file, "arrayBuffer", { value: async () => new TextEncoder().encode("# Review").buffer });
  return file;
}

it("imports a named standalone file through the runtime and clears the input for retry", async () => {
  render(<SkillsSection />);
  const button = screen.getByRole("button", { name: "Import SKILL.md file" }) as HTMLButtonElement;
  expect(button.disabled).toBe(true);
  fireEvent.change(screen.getByRole("textbox", { name: "Name for a standalone skill" }), { target: { value: "Review" } });
  expect(button.disabled).toBe(false);
  const input = screen.getByLabelText("Skill instructions file") as HTMLInputElement;
  fireEvent.change(input, { target: { files: [instructions()] } });
  await waitFor(() => expect(state.import).toHaveBeenCalledWith({ kind: "import", request: {
    name: "Review", files: [{ path: "SKILL.md", contentBase64: "IyBSZXZpZXc=" }],
  } }));
  await waitFor(() => expect(input.value).toBe(""));
  expect(screen.getByRole("button", { name: "Import skill folder" })).toBeTruthy();
});

it("reports validation failures without importing and allows choosing the same file again", async () => {
  render(<SkillsSection />);
  fireEvent.change(screen.getByRole("textbox"), { target: { value: "Review" } });
  const input = screen.getByLabelText("Skill instructions file");
  fireEvent.change(input, { target: { files: [instructions("README.md")] } });
  await waitFor(() => expect(screen.getByRole("alert").textContent).toContain("Choose a SKILL.md"));
  expect(state.import).not.toHaveBeenCalled();
  fireEvent.change(input, { target: { files: [instructions()] } });
  await waitFor(() => expect(state.import).toHaveBeenCalledOnce());
});

it("does not import when the picker is cancelled and disables import without a runtime", () => {
  state.available = false;
  render(<SkillsSection />);
  expect((screen.getByRole("button", { name: "Import skill folder" }) as HTMLButtonElement).disabled).toBe(true);
  fireEvent.change(screen.getByLabelText("Skill instructions file"), { target: { files: [] } });
  expect(state.import).not.toHaveBeenCalled();
});
