import { invoke } from "@tauri-apps/api/core";

export async function listLocalFontFamilies(): Promise<string[]> {
  return invoke<string[]>("list_local_font_families");
}
