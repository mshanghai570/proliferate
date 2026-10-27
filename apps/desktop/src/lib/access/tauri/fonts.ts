import { invoke } from "@tauri-apps/api/core";
import type { SystemFontInfo } from "@proliferate/product-client/host/desktop-bridge";

export type { SystemFontInfo };

// ---------------------------------------------------------------------------
// Low-level wrappers (1:1 with Rust commands)
// ---------------------------------------------------------------------------

export async function listSystemFonts(): Promise<SystemFontInfo[]> {
  const payload = await invoke<unknown>("list_system_fonts");
  if (!isSystemFontList(payload)) {
    throw new Error("Invalid list_system_fonts response.");
  }
  return payload;
}

function isSystemFontList(value: unknown): value is SystemFontInfo[] {
  if (!Array.isArray(value)) {
    return false;
  }
  return value.every((entry) => (
    entry !== null
    && typeof entry === "object"
    && typeof (entry as Record<string, unknown>).familyName === "string"
    && typeof (entry as Record<string, unknown>).displayName === "string"
  ));
}
