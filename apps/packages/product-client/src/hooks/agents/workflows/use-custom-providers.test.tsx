// @vitest-environment jsdom
import { act, renderHook } from "@testing-library/react";
import { beforeEach, expect, it, vi } from "vitest";
import type { HarnessAuthEditorApi } from "#product/hooks/agents/workflows/use-harness-auth-editor";
import { useCustomProviders } from "#product/hooks/agents/workflows/use-custom-providers";

const access = vi.hoisted(() => ({ create: { mutate: vi.fn() }, revoke: { mutate: vi.fn() } }));
vi.mock("#product/hooks/access/cloud/use-custom-provider-access", () => ({
  useCustomProviderAccess: () => access,
}));
beforeEach(() => vi.clearAllMocks());
const input = { title: "Local", kind: "openai_compatible" as const,
  value: { name: "Local", endpoint: "http://localhost:1234/v1", models: "model-a" } };

function editor() {
  return { busy: false, apiKeysQuery: { data: [] }, editorState: { rows: [] },
    addBoundApiKey: vi.fn(), } as unknown as HarnessAuthEditorApi;
}

it("waits for the selection write before closing the form", () => {
  const auth = editor();
  const { result } = renderHook(() => useCustomProviders(auth));
  act(() => result.current.show());
  act(() => result.current.submit(input));
  act(() => access.create.mutate.mock.calls[0][1].onSuccess({ id: "key-1" }));
  expect(result.current.open).toBe(true);
  expect(result.current.busy).toBe(true);
  expect(auth.addBoundApiKey).toHaveBeenCalledWith(null, "openai_compatible", "key-1", expect.any(Object));
  act(() => vi.mocked(auth.addBoundApiKey).mock.calls[0][3]!.onSuccess!());
  expect(result.current.open).toBe(false);
  expect(result.current.busy).toBe(false);
});

it("revokes an unbound entry and retains the form after a failed selection write", () => {
  const auth = editor();
  const { result } = renderHook(() => useCustomProviders(auth));
  act(() => result.current.show());
  act(() => result.current.submit(input));
  act(() => access.create.mutate.mock.calls[0][1].onSuccess({ id: "key-1" }));
  act(() => vi.mocked(auth.addBoundApiKey).mock.calls[0][3]!.onError!("Selection rejected"));
  expect(access.revoke.mutate).toHaveBeenCalledWith("key-1", expect.any(Object));
  act(() => access.revoke.mutate.mock.calls[0][1].onSuccess());
  expect(result.current.error).toBe("Selection rejected");
  expect(result.current.open).toBe(true);
  expect(result.current.busy).toBe(false);
});
