// @vitest-environment jsdom
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import type { DesktopAiProvidersBridge } from "@proliferate/product-client/host/desktop-bridge";
import type { ProductStorage } from "@proliferate/product-client/host/product-host";

const setup = vi.hoisted(() => ({
  desktop: null as { aiProviders: DesktopAiProvidersBridge } | null,
  storage: null as ProductStorage | null,
  storageContext: null as { storage: ProductStorage; captureException: ReturnType<typeof vi.fn> } | null,
}));
vi.mock("@proliferate/product-client/host/ProductHostProvider", () => ({
  useProductHost: () => ({ desktop: setup.desktop }),
}));
vi.mock("#product/hooks/persistence/facade/use-product-storage-context", () => ({
  useProductStorageContext: () => setup.storageContext,
}));

import { NativeProviderProfilesPane } from "#product/components/settings/panes/agents/native-providers/NativeProviderProfilesPane";

afterEach(cleanup);
beforeEach(() => {
  const values = new Map<string, string>();
  setup.storage = {
    getItem: vi.fn(async (key) => values.get(key) ?? null),
    setItem: vi.fn(async (key, value) => { values.set(key, value); }),
    removeItem: vi.fn(async (key) => { values.delete(key); }),
  };
  setup.storageContext = { storage: setup.storage, captureException: vi.fn() };
  setup.desktop = { aiProviders: {
    isSupported: () => true,
    promptForKey: vi.fn(async () => "saved"),
    hasKey: vi.fn(async () => true),
    deleteKey: vi.fn(async () => {}),
    discoverModels: vi.fn(async () => ({ models: [{ id: "local-model" }], refreshedAt: "2026-10-23T12:00:00Z" })),
  } };
});

describe("NativeProviderProfilesPane", () => {
  it("creates a local profile and mediates key setup without rendering the credential", async () => {
    render(<NativeProviderProfilesPane />);
    fireEvent.change(screen.getByLabelText("Provider name"), { target: { value: "Local server" } });
    fireEvent.change(screen.getByLabelText("Endpoint URL"), { target: { value: "http://localhost:1234/v1" } });
    fireEvent.click(screen.getByRole("button", { name: "Add provider" }));
    expect(await screen.findByText("Local server")).toBeTruthy();

    fireEvent.click(screen.getByRole("button", { name: "Add API key" }));
    await waitFor(() => expect(setup.desktop?.aiProviders.promptForKey).toHaveBeenCalledTimes(1));
    expect(setup.desktop?.aiProviders.promptForKey).toHaveBeenCalledWith(expect.any(String));
    expect(screen.queryByText(/secret-value/)).toBeNull();
    expect(await screen.findByText("API key saved in Keychain")).toBeTruthy();
  });

  it("shows cached models and a retryable discovery error without dropping the cache", async () => {
    render(<NativeProviderProfilesPane />);
    fireEvent.change(screen.getByLabelText("Provider name"), { target: { value: "Local server" } });
    fireEvent.change(screen.getByLabelText("Endpoint URL"), { target: { value: "http://localhost:1234/v1" } });
    fireEvent.click(screen.getByRole("button", { name: "Add provider" }));
    fireEvent.click(await screen.findByRole("button", { name: "Discover models" }));
    expect(await screen.findByText("local-model")).toBeTruthy();

    vi.mocked(setup.desktop!.aiProviders.discoverModels).mockRejectedValueOnce(new Error("Provider timed out."));
    fireEvent.click(screen.getByRole("button", { name: "Refresh models" }));
    expect((await screen.findByRole("alert")).textContent).toContain("Provider timed out.");
    expect(screen.getByText("local-model")).toBeTruthy();
  });

  it("retains the profile if Keychain refuses to delete its key", async () => {
    vi.mocked(setup.desktop!.aiProviders.deleteKey).mockRejectedValueOnce(new Error("Keychain is locked."));
    render(<NativeProviderProfilesPane />);
    await screen.findByText("No native provider profiles yet.");
    fireEvent.change(screen.getByLabelText("Provider name"), { target: { value: "Local server" } });
    fireEvent.change(screen.getByLabelText("Endpoint URL"), { target: { value: "http://localhost:1234/v1" } });
    fireEvent.click(screen.getByRole("button", { name: "Add provider" }));
    await screen.findByText("Local server");
    fireEvent.click(screen.getByRole("button", { name: "Add API key" }));
    await screen.findByText("API key saved in Keychain");
    fireEvent.click(screen.getByRole("button", { name: "Remove key" }));
    expect((await screen.findByRole("alert")).textContent).toContain("Keychain is locked.");
    expect(screen.getByText("Local server")).toBeTruthy();
  });
});
