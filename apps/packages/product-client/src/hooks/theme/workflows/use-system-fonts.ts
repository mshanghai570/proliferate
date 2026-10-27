import { useEffect, useState } from "react";
import { useProductHost } from "@proliferate/product-client/host/ProductHostProvider";

export interface SystemFontListState {
  fonts: SystemFontSupply[];
  status: "idle" | "loading" | "ready" | "unavailable" | "failed";
}

/** One installed font family, keyed on the CSS family name. */
export interface SystemFontSupply {
  familyName: string;
  displayName: string;
}

function useSystemFontList(): SystemFontListState {
  const { desktop } = useProductHost();
  const [state, setState] = useState<SystemFontListState>({
    fonts: [],
    status: "idle",
  });

  useEffect(() => {
    const bridge = desktop?.fonts ?? null;
    if (!bridge) {
      setState({ fonts: [], status: "unavailable" });
      return;
    }
    let cancelled = false;
    setState({ fonts: [], status: "loading" });
    bridge.listSystemFonts()
      .then((fonts) => {
        if (!cancelled) {
          setState({ fonts, status: "ready" });
        }
      })
      .catch(() => {
        if (!cancelled) {
          setState({ fonts: [], status: "failed" });
        }
      });
    return () => {
      cancelled = true;
    };
  }, [desktop]);

  return state;
}

export { useSystemFontList };
