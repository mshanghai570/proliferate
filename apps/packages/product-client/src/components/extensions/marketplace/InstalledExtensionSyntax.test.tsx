// @vitest-environment jsdom
import { AnyHarnessRuntime } from "@anyharness/sdk-react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { InstalledExtensions } from "./InstalledExtensions";
import { ImportedSyntaxThemePreference } from "#product/components/settings/panes/ImportedSyntaxThemePreference";
import { USER_PREFERENCE_DEFAULTS } from "#product/lib/domain/preferences/user/model";
import { useUserPreferencesStore } from "#product/stores/preferences/user-preferences-store";
const extension = { namespace: "example", name: "theme", version: "1.0.0", sha256: "a".repeat(64), state: "installed", active: false, errorCode: null };
const theme = { label: "Night", colorMode: "dark", available: true, colors: { "editor.background": "#101820", "editor.foreground": "#ffffff" }, rules: [{ scopes: ["comment"], foreground: "#AA55CC" }] };
afterEach(() => { cleanup(); useUserPreferencesStore.setState({ ...USER_PREFERENCE_DEFAULTS }); });
function view(fetch: typeof globalThis.fetch, scope = "actor-a", client = new QueryClient({ defaultOptions: { queries: { retry: false } } })) {
  return <QueryClientProvider client={client}><AnyHarnessRuntime runtimeUrl="http://runtime.test" cacheScopeKey={scope} fetch={fetch}><InstalledExtensions appColorsAvailable /></AnyHarnessRuntime></QueryClientProvider>;
}
describe("installed syntax themes", () => {
  it("reads only on demand, explicitly applies colors, and resets syntax without removing the package", async () => {
    const fetch = vi.fn(async (url: RequestInfo | URL) => new Response(JSON.stringify(String(url).endsWith("/syntax-themes")
      ? { themes: [theme, { ...theme, label: "Unsupported", available: false }], truncated: false } : { extensions: [extension] })));
    render(view(fetch));
    await screen.findByText("example.theme");
    expect(fetch.mock.calls.some(([url]) => String(url).endsWith("/syntax-themes"))).toBe(false);
    fireEvent.click(screen.getByRole("button", { name: "Syntax themes" }));
    await screen.findByRole("button", { name: "Use Night syntax theme" });
    expect(useUserPreferencesStore.getState().importedSyntaxTheme).toBeNull();
    expect(screen.queryByRole("button", { name: "Use Unsupported syntax theme" })).toBeNull();
    fireEvent.click(screen.getByRole("button", { name: "Use Night syntax theme" }));
    expect(useUserPreferencesStore.getState().importedSyntaxTheme?.rules).toEqual(theme.rules);
    expect(useUserPreferencesStore.getState().importedAppTheme?.background).toBe("#101820");
    render(<ImportedSyntaxThemePreference />);
    fireEvent.click(screen.getByRole("button", { name: "Reset syntax colors" }));
    expect(useUserPreferencesStore.getState().importedSyntaxTheme).toBeNull();
    expect(screen.getByText("example.theme")).toBeTruthy();
  });
  it("drops another actor's controls and shows bounded errors on older runtimes", async () => {
    let actor = "a";
    const fetch = vi.fn(async (url: RequestInfo | URL) => String(url).endsWith("/syntax-themes")
      ? actor === "a" ? new Response(JSON.stringify({ themes: [theme], truncated: false })) : new Response(JSON.stringify({ title: "private marker", status: 404 }), { status: 404 })
      : new Response(JSON.stringify({ extensions: [extension] })));
    const client = new QueryClient({ defaultOptions: { queries: { retry: false } } });
    const rendered = render(view(fetch, "actor-a", client));
    fireEvent.click(await screen.findByRole("button", { name: "Syntax themes" }));
    await screen.findByRole("button", { name: "Use Night syntax theme" });
    actor = "b"; rendered.rerender(view(fetch, "actor-b", client));
    expect(screen.queryByRole("button", { name: "Use Night syntax theme" })).toBeNull();
    fireEvent.click(await screen.findByRole("button", { name: "Syntax themes" }));
    await screen.findByRole("alert");
    expect(screen.queryByText(/private marker/)).toBeNull();
    expect(screen.getByRole("button", { name: "Retry" })).toBeTruthy();
  });
});
