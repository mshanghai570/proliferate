// @vitest-environment jsdom
import { act, renderHook } from "@testing-library/react";
import { beforeEach, expect, it, vi } from "vitest";
import { useLocalProviderEditor } from "#product/hooks/agents/workflows/use-local-provider-editor";
const access = vi.hoisted(() => ({ save: vi.fn(), toggle: vi.fn(), remove: vi.fn(), query: { refetch: vi.fn() } }));
vi.mock("#product/hooks/access/anyharness/use-local-providers", () => ({ useLocalProviders: () => access }));
beforeEach(() => vi.clearAllMocks());
const input = { title: "My models", kind: "openai_compatible" as const, value: { name: "My models", endpoint: "http://localhost:1234/v1", models: "model-a\nmodel-b", apiFormat: "chat", apiKey: "private-marker" } };

it("saves directly to the runtime and closes the credential form after success", async () => {
  access.save.mockResolvedValue(undefined);
  const { result } = renderHook(() => useLocalProviderEditor());
  act(() => result.current.show());
  await act(async () => { result.current.submit(input); });
  expect(access.save).toHaveBeenCalledWith({ name: "My models", endpoint: "http://localhost:1234/v1", models: ["model-a", "model-b"], apiFormat: "chat", apiKey: "private-marker" });
  expect(result.current.open).toBe(false);
  expect(result.current.notice).toContain("Saved on this Mac");
});
it("prevents duplicate writes before React rerenders", async () => {
  let finish!: () => void;
  access.save.mockImplementation(() => new Promise<void>((resolve) => { finish = resolve; }));
  const { result } = renderHook(() => useLocalProviderEditor());
  act(() => { result.current.submit(input); result.current.submit(input); });
  expect(access.save).toHaveBeenCalledOnce();
  await act(async () => { finish(); });
});
it("retains the form and uses authored errors after a failed save", async () => {
  access.save.mockRejectedValue(new Error("private-marker"));
  const { result } = renderHook(() => useLocalProviderEditor());
  act(() => result.current.show());
  await act(async () => { result.current.submit(input); });
  expect(result.current.open).toBe(true);
  expect(result.current.error).not.toContain("private-marker");
});
it("requires removal selection and closes the confirmation after a failed removal", async () => {
  access.remove.mockRejectedValue(new Error("unavailable"));
  const { result } = renderHook(() => useLocalProviderEditor());
  act(() => result.current.confirmRemove());
  expect(access.remove).not.toHaveBeenCalled();
  act(() => result.current.requestRemove("one"));
  await act(async () => { result.current.confirmRemove(); });
  expect(access.remove).toHaveBeenCalledWith("one");
  expect(result.current.removeId).toBeNull();
});
