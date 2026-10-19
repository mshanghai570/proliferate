import { afterEach, beforeEach, expect, it, vi } from "vitest";

beforeEach(() => { vi.resetModules(); vi.stubEnv("VITE_PROLIFERATE_DESKTOP_REDIRECT_SCHEME", undefined); });
afterEach(() => { vi.unstubAllEnvs(); vi.resetModules(); });

it("uses the registered preview callback scheme for pending sign-in", async () => {
  vi.stubEnv("VITE_PROLIFERATE_DESKTOP_REDIRECT_SCHEME", "proliferate-local");
  const auth = await import("./proliferate-auth-redirect");
  expect(auth.DESKTOP_AUTH_REDIRECT_URI).toBe("proliferate-local://auth/callback");
  const pending = auth.createPendingGitHubDesktopAuth();
  expect(pending.redirect_uri).toBe(auth.DESKTOP_AUTH_REDIRECT_URI);
  expect(pending.state.length).toBeGreaterThan(20);
  expect(pending.code_verifier.length).toBeGreaterThan(40);
  expect(auth.parseDesktopAuthCallback(`${pending.redirect_uri}?state=state&code=code`)).toMatchObject({ state: "state", code: "code" });
});
it("preserves the production callback when no override is set", async () => {
  const auth = await import("./proliferate-auth-redirect");
  expect(auth.DESKTOP_AUTH_REDIRECT_URI).toBe("proliferate://auth/callback");
});
it("refuses unsupported build overrides rather than sending auth to another scheme", async () => {
  vi.stubEnv("VITE_PROLIFERATE_DESKTOP_REDIRECT_SCHEME", "proliferate-preview");
  await expect(import("./proliferate-auth-redirect")).rejects.toThrow("Unsupported desktop auth callback scheme");
});
it("keeps callback shape and scheme validation", async () => {
  const auth = await import("./proliferate-auth-redirect");
  for (const url of ["https://auth/callback?state=x&code=y", "proliferate-preview://auth/callback?state=x&code=y",
    "proliferate-local://other/callback?state=x&code=y", "proliferate-local://auth/other?state=x&code=y", "proliferate-local://auth/callback?code=y"]) {
    expect(auth.parseDesktopAuthCallback(url)).toBeNull();
  }
});
