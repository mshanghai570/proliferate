import { useEffect, useRef, useState } from "react";
import { useExtensionMetadata, useExtensionManifest } from "#product/hooks/access/extensions/use-extension-registry";
import { assessExtensionCompatibility, EMPTY_EXTENSION_HOST_CAPABILITIES } from "#product/lib/domain/extensions/extension-compatibility";
import type { ExtensionRef } from "#product/lib/domain/extensions/extension-models";

export type ExtensionDetailTab = "About" | "Changelog" | "Versions" | "Capabilities";

export function useExtensionDetail(ref: ExtensionRef) {
  const headingRef = useRef<HTMLHeadingElement>(null);
  const [tab, setTab] = useState<ExtensionDetailTab>("About");
  const metadata = useExtensionMetadata(ref);
  const manifest = useExtensionManifest(metadata.data?.fileUrls.manifest ?? null);
  useEffect(() => {
    if (metadata.data) headingRef.current?.focus();
  }, [metadata.data]);
  const compatibility = manifest.data
    ? assessExtensionCompatibility(manifest.data, EMPTY_EXTENSION_HOST_CAPABILITIES)
    : null;
  return { tab, setTab, metadata, manifest, compatibility, headingRef };
}
