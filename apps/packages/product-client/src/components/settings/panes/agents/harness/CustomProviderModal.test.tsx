// @vitest-environment jsdom
import type { ReactNode } from "react";
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, expect, it, vi } from "vitest";
import { CustomProviderModal } from "#product/components/settings/panes/agents/harness/CustomProviderModal";

vi.mock("#product/primitives/patterns/ModalShell", () => ({
  ModalShell: ({ children }: { children: ReactNode }) => <div>{children}</div>,
}));
afterEach(cleanup);

it("saves a keyless connection with exact endpoint and model IDs", () => {
  const onSubmit = vi.fn();
  render(<CustomProviderModal onClose={vi.fn()} onSubmit={onSubmit} busy={false} error={null} />);
  fireEvent.change(screen.getByLabelText("Name"), { target: { value: "Local models" } });
  fireEvent.change(screen.getByLabelText("API base URL"), { target: { value: "http://localhost:1234/v1" } });
  fireEvent.change(screen.getByLabelText("Model IDs"), { target: { value: "org/model-a\nmodel-b" } });
  fireEvent.click(screen.getByRole("button", { name: "Add provider" }));
  expect(onSubmit).toHaveBeenCalledWith({
    title: "Local models", kind: "openai_compatible",
    value: { name: "Local models", endpoint: "http://localhost:1234/v1", models: "org/model-a\nmodel-b", apiFormat: "chat" },
  });
});

it("keeps errors visible and prevents duplicate submissions", () => {
  render(<CustomProviderModal onClose={vi.fn()} onSubmit={vi.fn()} busy error="Could not save provider. Try again." />);
  expect(screen.getByRole("alert").textContent).toContain("Could not save provider");
  expect((screen.getByRole("button", { name: "Add provider" }) as HTMLButtonElement).disabled).toBe(true);
});
