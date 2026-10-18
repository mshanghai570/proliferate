// @vitest-environment jsdom
import { AnyHarnessRuntime } from "@anyharness/sdk-react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { InstalledExtensions } from "./InstalledExtensions";
import { USER_PREFERENCE_DEFAULTS } from "#product/lib/domain/preferences/user/model";
import { useUserPreferencesStore } from "#product/stores/preferences/user-preferences-store";

const extension = { namespace: "example", name: "theme", version: "1.0.0", sha256: "a".repeat(64), state: "installed", active: false, errorCode: null };
const theme = { label: "Midnight", colorMode: "dark", available: true, colors: { "editor.background": "#101820", "editor.foreground": "#ffffff" } };
afterEach(() => { cleanup(); useUserPreferencesStore.setState({ ...USER_PREFERENCE_DEFAULTS }); });

function view(fetch: typeof globalThis.fetch, scope = "actor-a", client = new QueryClient({ defaultOptions: { queries: { retry: false } } })) {
  return <QueryClientProvider client={client}><AnyHarnessRuntime runtimeUrl="http://runtime.test" cacheScopeKey={scope} fetch={fetch}><InstalledExtensions appColorsAvailable /></AnyHarnessRuntime></QueryClientProvider>;
}

describe("installed app color themes", () => {
  it("loads on demand, applies readable colors, and refuses unusable palettes", async () => {
    const fetch = vi.fn(async (url: RequestInfo | URL) => new Response(JSON.stringify(String(url).endsWith("/color-themes")
      ? { themes: [theme, { ...theme, label: "Unreadable", colors: { "editor.background": "#101820", "editor.foreground": "#111111" } }], truncated: false }
      : { extensions: [extension] })));
    render(view(fetch));
    const showColors = await screen.findByRole("button", { name: "App color themes" }, { timeout: 10_000 });
    expect(fetch.mock.calls.some(([url]) => String(url).endsWith("/color-themes"))).toBe(false);
    fireEvent.click(showColors);
    fireEvent.click(await screen.findByRole("button", { name: "Use Midnight app colors" }));
    expect(screen.queryByRole("button", { name: "Use Unreadable app colors" })).toBeNull();
    expect(useUserPreferencesStore.getState().importedAppTheme).toMatchObject({ name: "Midnight", background: "#101820", foreground: "#FFFFFF" });
    expect(useUserPreferencesStore.getState().colorMode).toBe("dark");
    expect(await screen.findByText(/Applied Midnight/)).toBeTruthy();
    expect(fetch.mock.calls.filter(([url]) => String(url).endsWith("/color-themes"))).toHaveLength(1);
  });

  it("does not retain another actor's palette controls", async () => {
    let actor = "a";
    const fetch = vi.fn(async (url: RequestInfo | URL) => new Response(JSON.stringify(String(url).endsWith("/color-themes")
      ? { themes: actor === "a" ? [theme] : [], truncated: false }
      : { extensions: [extension] })));
    const client = new QueryClient({ defaultOptions: { queries: { retry: false } } });
    const rendered = render(view(fetch, "actor-a", client));
    fireEvent.click(await screen.findByRole("button", { name: "App color themes" }));
    await screen.findByRole("button", { name: "Use Midnight app colors" });
    actor = "b";
    rendered.rerender(view(fetch, "actor-b", client));
    fireEvent.click(await screen.findByRole("button", { name: "App color themes" }));
    await screen.findByText("This package declares no color themes.");
    expect(screen.queryByRole("button", { name: "Use Midnight app colors" })).toBeNull();
  });

  it("shows authored feedback for older runtimes without exposing response details", async () => {
    const fetch = vi.fn(async (url: RequestInfo | URL) => String(url).endsWith("/color-themes")
      ? new Response(JSON.stringify({ title: "private filesystem marker", status: 404 }), { status: 404 })
      : new Response(JSON.stringify({ extensions: [extension] })));
    render(view(fetch));
    fireEvent.click(await screen.findByRole("button", { name: "App color themes" }));
    await screen.findByRole("alert");
    expect(screen.queryByText(/private filesystem marker/)).toBeNull();
    expect(screen.queryByRole("button", { name: "Use Midnight app colors" })).toBeNull();
  });
});
