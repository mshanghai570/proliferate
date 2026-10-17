// @vitest-environment jsdom
import { AnyHarnessRuntime } from "@anyharness/sdk-react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { useExtensionInstallation } from "#product/hooks/access/extensions/use-extension-installation";
import { useInstalledExtensionsAccess } from "#product/hooks/access/extensions/use-installed-extensions";

function InstallationHarness() {
  const { install } = useExtensionInstallation();
  const installed = useInstalledExtensionsAccess();
  return (
    <>
      <button onClick={() => install.mutate({ namespace: "example", name: "theme", version: "1.0.0" })}>
        Install package
      </button>
      <p>{installed.query.data?.extensions.length ?? 0} installed</p>
    </>
  );
}

describe("extension installation access", () => {
  it("refreshes installed state for the originating runtime and actor after installation", async () => {
    let installed = false;
    const fetch = vi.fn(async (_url: RequestInfo | URL, options?: RequestInit) => {
      if (options?.method === "POST") {
        installed = true;
        return new Response(JSON.stringify({ namespace: "example", name: "theme", version: "1.0.0", active: false, state: "installed", sha256: "a".repeat(64), errorCode: null }), { status: 201 });
      }
      return new Response(JSON.stringify({ extensions: installed ? [{ namespace: "example", name: "theme", version: "1.0.0", active: false, state: "installed", sha256: "a".repeat(64), errorCode: null }] : [] }));
    });
    const queryClient = new QueryClient({ defaultOptions: { queries: { retry: false }, mutations: { retry: false } } });

    render(
      <QueryClientProvider client={queryClient}>
        <AnyHarnessRuntime runtimeUrl="http://runtime.test" cacheScopeKey="actor-a" fetch={fetch}>
          <InstallationHarness />
        </AnyHarnessRuntime>
      </QueryClientProvider>,
    );

    await screen.findByText("0 installed");
    fireEvent.click(screen.getByRole("button", { name: "Install package" }));
    await screen.findByText("1 installed");
    expect(fetch.mock.calls.filter(([, options]) => options?.method === "GET")).toHaveLength(2);
    expect(fetch.mock.calls.filter(([, options]) => options?.method === "POST")).toHaveLength(1);
  });
});
