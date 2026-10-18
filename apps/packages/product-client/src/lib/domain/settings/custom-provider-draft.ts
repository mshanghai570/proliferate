export interface CustomProviderDraft {
  name: string;
  endpoint: string;
  apiKey: string;
  models: string;
  apiFormat: string;
}

export function validateCustomProviderDraft(draft: CustomProviderDraft): string | null {
  const name = draft.name.trim();
  const endpoint = draft.endpoint.trim();
  if (!name || name.length > 255 || /[\u0000-\u001f\u007f]/u.test(name)) {
    return "Enter a provider name with 1–255 characters and no control characters.";
  }
  if (Object.values(draft).some((value) => /\{(?:env|file):/u.test(value))) {
    return "Enter literal values. Environment and file substitutions are not supported.";
  }
  try {
    const url = new URL(endpoint);
    if (!/^https?:\/\//u.test(endpoint) || !/^https?:$/u.test(url.protocol) || !url.hostname || url.username || url.password
      || url.search || url.hash || url.port === "0" || /[\s{}\\]/u.test(endpoint)) {
      return "Enter an HTTP or HTTPS API base URL without credentials, a query, or a fragment.";
    }
  } catch {
    return "Enter a valid HTTP or HTTPS API base URL, including its API base path.";
  }
  const models = [...new Set(draft.models.split(/\r?\n/u).map((id) => id.trim()).filter(Boolean))];
  if (!models.length || models.length > 100 || models.some((id) => /[\s{}\u0000-\u001f\u007f]/u.test(id))) {
    return "Enter 1–100 model IDs, one per line, without spaces or braces.";
  }
  if (/[\r\n\u0000]/u.test(draft.apiKey)) return "Enter an API key without line breaks or null characters.";
  if (draft.apiFormat !== "chat" && draft.apiFormat !== "responses") return "Choose Chat Completions or Responses.";
  return null;
}
