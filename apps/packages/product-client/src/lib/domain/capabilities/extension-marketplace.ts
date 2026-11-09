/** Desktop sets this at build time; Web retains an opt-in preview. */
export function isExtensionMarketplaceEnabled(): boolean {
  return import.meta.env.VITE_EXTENSION_MARKETPLACE === "1";
}
