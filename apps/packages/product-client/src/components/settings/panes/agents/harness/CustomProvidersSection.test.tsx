// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, expect, it, vi } from "vitest";
import type { HarnessAuthEditorApi } from "#product/hooks/agents/workflows/use-harness-auth-editor";
import { CustomProvidersSection } from "#product/components/settings/panes/agents/harness/CustomProvidersSection";

vi.mock("#product/hooks/agents/workflows/use-custom-providers", () => ({
  useCustomProviders: () => ({ rows: [{ uid: "row-1", title: "Local models", enabled: true }],
    open: false, busy: false, error: null, show: vi.fn() }),
}));
afterEach(cleanup);

it("lets users disable and remove a connection after policy disallows it", () => {
  const editor = { authReady: true, apiKeyDisallowed: true, harnessDisallowed: true,
    handleRowEnabledToggle: vi.fn(), handleRemoveRow: vi.fn(), } as unknown as HarnessAuthEditorApi;
  render(<CustomProvidersSection editor={editor} />);
  fireEvent.click(screen.getByRole("switch", { name: "Enable Local models" }));
  expect(editor.handleRowEnabledToggle).toHaveBeenCalledWith("row-1", false);
  fireEvent.click(screen.getByRole("button", { name: "Remove Local models" }));
  expect(editor.handleRemoveRow).toHaveBeenCalledWith("row-1");
  expect((screen.getByRole("button", { name: "Add OpenAI-compatible provider" }) as HTMLButtonElement).disabled).toBe(true);
});
