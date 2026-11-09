/**
 * Failure vocabulary for extension registry access.
 *
 * Expected outcomes stay data where the caller can branch on them: the error
 * carries a stable `code` rather than a message to be sniffed, and the message
 * is for humans only. `retryAfterSeconds` is present only for `rate-limited`.
 */

export type ExtensionRegistryErrorCode =
  | "not-found"
  | "rate-limited"
  | "http-error"
  | "malformed-response"
  | "network-error";

export class ExtensionRegistryError extends Error {
  readonly code: ExtensionRegistryErrorCode;
  /** HTTP status when the failure came from a response. */
  readonly status: number | null;
  /** Seconds the registry asked us to wait, for `rate-limited`. */
  readonly retryAfterSeconds: number | null;

  constructor(
    code: ExtensionRegistryErrorCode,
    message: string,
    options: { status?: number | null; retryAfterSeconds?: number | null } = {},
  ) {
    super(message);
    this.name = "ExtensionRegistryError";
    this.code = code;
    this.status = options.status ?? null;
    this.retryAfterSeconds = options.retryAfterSeconds ?? null;
  }
}

/** True when the caller may retry the same request unchanged. */
export function isRetryableRegistryError(error: unknown): boolean {
  return error instanceof ExtensionRegistryError && error.code === "rate-limited";
}
