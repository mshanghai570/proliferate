/** Discovery stays opt-in until installation and contribution loading are ready. */
export function isExtensionMarketplaceEnabled(): boolean {
  return import.meta.env.VITE_EXTENSION_MARKETPLACE === "1";
}
