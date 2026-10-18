import { describe, expect, it } from "vitest";
import { validateCustomProviderDraft } from "#product/lib/domain/settings/custom-provider-draft";

const valid = { name: "My server", endpoint: "http://localhost:1234/v1", apiKey: "", models: "org/model", apiFormat: "chat" };

describe("custom provider draft", () => {
  it("accepts keyless local and authenticated hosted endpoints with exact model IDs", () => {
    expect(validateCustomProviderDraft(valid)).toBeNull();
    expect(validateCustomProviderDraft({ ...valid, endpoint: "https://models.example/api/v1/", apiKey: "secret", apiFormat: "responses", models: "org/model:v2\nmodel-b" })).toBeNull();
  });
  it.each(["file:///tmp/models", "ftp://example.com", "https:example.com", "https://user:secret@example.com/v1", "https://example.com/v1?key=secret", "https://example.com/#fragment", "http://localhost:0/v1", "http://localhost:99999/v1", "https://example.com/a b", "https://example.com/{env:URL}"])("rejects unsafe or invalid endpoint %s", (endpoint) => {
    expect(validateCustomProviderDraft({ ...valid, endpoint })).not.toBeNull();
  });
  it("rejects substitutions and invalid model lists", () => {
    for (const models of ["", "model with space", "{file:config}", Array.from({ length: 101 }, (_, index) => `model-${index}`).join("\n")]) {
      expect(validateCustomProviderDraft({ ...valid, models })).not.toBeNull();
    }
    expect(validateCustomProviderDraft({ ...valid, models: "model\nmodel" })).toBeNull();
    expect(validateCustomProviderDraft({ ...valid, apiKey: "{env:KEY}" })).not.toBeNull();
  });
  it("rejects malformed names, keys, and API formats", () => {
    expect(validateCustomProviderDraft({ ...valid, name: " " })).not.toBeNull();
    expect(validateCustomProviderDraft({ ...valid, name: "a".repeat(256) })).not.toBeNull();
    expect(validateCustomProviderDraft({ ...valid, apiKey: "secret\nvalue" })).not.toBeNull();
    expect(validateCustomProviderDraft({ ...valid, apiFormat: "unknown" })).not.toBeNull();
  });
});
