// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import { AccountSignInSection } from "./AccountSignInSection";

const state = vi.hoisted(() => ({ reachable: true, github: true, password: true, checking: false,
  error: null as string | null, githubSignIn: vi.fn(), passwordSignIn: vi.fn(), health: vi.fn(), methods: vi.fn(), availability: vi.fn() }));
vi.mock("#product/hooks/auth/workflows/use-github-sign-in", () => ({ useGitHubSignIn: () => ({
  signInAvailable: state.github, signInChecking: state.checking, submitting: false, error: state.error, signIn: state.githubSignIn, clearError: vi.fn(),
}) }));
vi.mock("#product/hooks/auth/workflows/use-password-sign-in", () => ({ usePasswordSignIn: () => ({
  signInAvailable: state.password, signInChecking: state.checking, submitting: false, error: null, signIn: state.passwordSignIn, clearError: vi.fn(),
}) }));
vi.mock("#product/hooks/access/cloud/use-control-plane-health", () => ({ useControlPlaneHealth: () => ({ data: state.reachable, isFetching: false, refetch: state.health }) }));
vi.mock("#product/hooks/access/cloud/auth/use-auth-methods", () => ({ useDesktopAuthMethods: () => ({ refetch: state.methods }) }));
vi.mock("#product/hooks/access/cloud/auth/use-github-auth-availability", () => ({ useGitHubDesktopAuthAvailability: () => ({ refetch: state.availability }) }));
afterEach(cleanup);
beforeEach(() => {
  vi.clearAllMocks(); state.reachable = true; state.github = true; state.password = true; state.checking = false; state.error = null;
  state.githubSignIn.mockResolvedValue(undefined); state.passwordSignIn.mockResolvedValue(undefined); state.health.mockResolvedValue({ data: true });
});
it("offers the server's GitHub and email login methods from Account settings", async () => {
  render(<AccountSignInSection />);
  fireEvent.click(screen.getByRole("button", { name: "Continue with GitHub" }));
  expect(state.githubSignIn).toHaveBeenCalledOnce();
  fireEvent.change(screen.getByLabelText("Email"), { target: { value: "user@example.test" } });
  fireEvent.change(screen.getByLabelText("Password"), { target: { value: "example-password" } });
  fireEvent.click(screen.getByRole("button", { name: "Sign in" }));
  expect(state.passwordSignIn).toHaveBeenCalledWith("user@example.test", "example-password");
});
it("shows service unavailability and keeps a retry path without accepting credentials", async () => {
  state.reachable = false;
  render(<AccountSignInSection />);
  expect(screen.getByRole("status").textContent).toContain("Local agents remain available");
  expect(screen.queryByLabelText("Password")).toBeNull();
  expect(screen.queryByRole("button", { name: "Continue with GitHub" })).toBeNull();
  state.health.mockResolvedValue({ data: false });
  fireEvent.click(screen.getByRole("button", { name: "Retry account connection" }));
  await waitFor(() => expect(state.health).toHaveBeenCalledOnce());
  expect(state.methods).not.toHaveBeenCalled();
});
it("refreshes supported login methods after the server recovers", async () => {
  state.reachable = false; render(<AccountSignInSection />);
  fireEvent.click(screen.getByRole("button", { name: "Retry account connection" }));
  await waitFor(() => expect(state.methods).toHaveBeenCalledOnce());
  expect(state.availability).toHaveBeenCalledOnce();
});
it("does not invent unavailable server login methods", () => {
  state.github = false; render(<AccountSignInSection />);
  expect(screen.queryByRole("button", { name: "Continue with GitHub" })).toBeNull();
  expect(screen.getByLabelText("Password")).toBeTruthy();
});
it("shows authored sign-in errors and consumes workflow rejections", async () => {
  state.error = "Sign-in could not be completed."; state.githubSignIn.mockRejectedValue(new Error("failed"));
  render(<AccountSignInSection />); fireEvent.click(screen.getByRole("button", { name: "Continue with GitHub" }));
  await waitFor(() => expect(state.githubSignIn).toHaveBeenCalledOnce());
  expect(screen.getByRole("alert").textContent).toBe("Sign-in could not be completed.");
});
