// @vitest-environment jsdom
import { act, cleanup, renderHook, waitFor } from "@testing-library/react";
import { afterEach, expect, it, vi } from "vitest";
import { useHighlightedLines } from "./use-highlighted-lines";
import { useUserPreferencesStore } from "#product/stores/preferences/user-preferences-store";
import { USER_PREFERENCE_DEFAULTS } from "#product/lib/domain/preferences/user/model";
import { sanitizeImportedSyntaxTheme } from "#product/lib/domain/preferences/imported-syntax-theme";
const mode = vi.hoisted(() => ({ value: "dark" }));
vi.mock("#product/hooks/theme/derived/use-resolved-mode", () => ({ useResolvedMode: () => mode.value }));
afterEach(() => { cleanup(); mode.value = "dark"; useUserPreferencesStore.setState({ ...USER_PREFERENCE_DEFAULTS }); });
it("updates existing source highlighting when a syntax theme is applied, its mode changes, or it is reset", async () => {
  const theme = sanitizeImportedSyntaxTheme({ name: "Night", type: "dark", foreground: "#FFFFFF", background: "#101820",
    rules: [{ scopes: ["comment"], foreground: "#AA55CC" }] })!;
  const view = renderHook(() => useHighlightedLines("// comment", "javascript"));
  await waitFor(() => expect(view.result.current).not.toBeNull(), { timeout: 15000 });
  const baseline = view.result.current;
  act(() => useUserPreferencesStore.setState({ importedSyntaxTheme: theme }));
  await waitFor(() => expect(view.result.current?.[0].some((token) => token.color?.toUpperCase() === "#AA55CC")).toBe(true));
  mode.value = "light"; view.rerender();
  await waitFor(() => expect(view.result.current?.[0].some((token) => token.color?.toUpperCase() === "#AA55CC")).toBe(false));
  mode.value = "dark";
  act(() => useUserPreferencesStore.setState({ importedSyntaxTheme: null }));
  view.rerender();
  await waitFor(() => expect(view.result.current).toEqual(baseline));
}, 20000);
