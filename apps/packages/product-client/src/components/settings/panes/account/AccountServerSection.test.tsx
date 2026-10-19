// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import { AccountServerSection } from "./AccountServerSection";

const state = vi.hoisted(() => ({ available: true, connectedServerHost: "127.0.0.1:8000" as string | null,
  step: "closed", open: vi.fn(), openForUrl: vi.fn() }));
vi.mock("#product/hooks/auth/workflows/use-connect-server", () => ({ useConnectServer: () => state }));
vi.mock("#product/components/auth/ConnectServerDialog", () => ({ ConnectServerDialog: () => <div>Server connection dialog</div> }));
afterEach(cleanup);
beforeEach(() => { vi.clearAllMocks(); state.available = true; state.connectedServerHost = "127.0.0.1:8000"; state.step = "closed"; });
it("opens the existing validation and trust-confirm flow for Proliferate hosted", () => {
  render(<AccountServerSection />);
  fireEvent.click(screen.getByRole("button", { name: "Use Proliferate hosted service" }));
  expect(state.openForUrl).toHaveBeenCalledWith("https://app.proliferate.com/api");
  fireEvent.click(screen.getByRole("button", { name: "Connect to a server" }));
  expect(state.open).toHaveBeenCalledOnce();
});
it("shows the hosted selection without an unnecessary switch action", () => {
  state.connectedServerHost = null; render(<AccountServerSection />);
  expect(screen.getByText(/Proliferate hosted service/)).toBeTruthy();
  expect(screen.queryByRole("button", { name: "Use Proliferate hosted service" })).toBeNull();
});
it("does not offer native server switching on unsupported hosts", () => {
  state.available = false; render(<AccountServerSection />);
  expect(screen.queryByRole("button")).toBeNull();
});
