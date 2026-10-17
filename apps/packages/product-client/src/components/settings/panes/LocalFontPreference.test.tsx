// @vitest-environment jsdom

import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, expect, it } from "vitest";
import { LocalFontPreference } from "#product/components/settings/panes/LocalFontPreference";
import { USER_PREFERENCE_DEFAULTS } from "#product/lib/domain/preferences/user/model";
import { useUserPreferencesStore } from "#product/stores/preferences/user-preferences-store";

beforeEach(() => {
  useUserPreferencesStore.setState(USER_PREFERENCE_DEFAULTS);
});
afterEach(cleanup);

it("saves an installed font family name and resets to the built-in stack", () => {
  render(<LocalFontPreference preferenceKey="codeFontFamily" label="Code font" />);
  const input = screen.getByRole("textbox", { name: "Code font" });

  fireEvent.change(input, { target: { value: "JetBrains Mono" } });
  fireEvent.blur(input);
  expect(useUserPreferencesStore.getState().codeFontFamily).toBe("JetBrains Mono");

  fireEvent.click(screen.getByRole("button", { name: "Reset code font" }));
  expect(useUserPreferencesStore.getState().codeFontFamily).toBeNull();
  expect((input as HTMLInputElement).value).toBe("");
});

it("keeps the previous font when the entered family has invalid syntax", () => {
  useUserPreferencesStore.getState().set("codeFontFamily", "Avenir Next");
  render(<LocalFontPreference preferenceKey="codeFontFamily" label="Code font" />);
  const input = screen.getByRole("textbox", { name: "Code font" });

  fireEvent.change(input, { target: { value: 'Bad"; color: red' } });
  fireEvent.blur(input);

  expect(useUserPreferencesStore.getState().codeFontFamily).toBe("Avenir Next");
  expect(screen.getByRole("alert").textContent).toContain("letters, numbers");
});
