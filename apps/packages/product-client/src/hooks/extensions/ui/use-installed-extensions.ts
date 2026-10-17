import { useEffect, useRef, useState } from "react";
import { AnyHarnessError, type InstalledExtensionSummary } from "@anyharness/sdk";
import { useInstalledExtensionsAccess } from "#product/hooks/access/extensions/use-installed-extensions";
import { EXTENSION_MANAGEMENT_COPY as copy } from "#product/copy/extensions/management";

function errorMessage(error: unknown): string {
  if (error instanceof AnyHarnessError) {
    if (error.problem.status === 404 || error.problem.status === 405) return copy.unavailable;
    if (error.problem.status === 401 || error.problem.status === 403) return copy.forbidden;
    if (error.problem.code === "EXTENSION_PACKAGE_INVALID") return copy.conflict;
  }
  return copy.failed;
}

export function useInstalledExtensions() {
  const access = useInstalledExtensionsAccess();
  const [selection, setSelection] = useState<{
    scope: string;
    extension: InstalledExtensionSummary;
  } | null>(null);
  const pending = selection?.scope === access.scope ? selection.extension : null;
  const refreshRef = useRef<HTMLButtonElement>(null);
  const confirmRef = useRef<HTMLButtonElement>(null);
  const triggerRef = useRef<HTMLButtonElement | null>(null);
  const restoreFocus = useRef(false);
  useEffect(() => {
    setSelection(null);
    restoreFocus.current = false;
    triggerRef.current = null;
  }, [access.scope]);
  useEffect(() => {
    if (pending) confirmRef.current?.focus();
    else if (restoreFocus.current) {
      restoreFocus.current = false;
      (triggerRef.current?.isConnected ? triggerRef.current : refreshRef.current)?.focus();
    }
  }, [pending]);
  const completed = access.uninstall.isSuccess && access.uninstall.variables?.scope === access.scope;
  useEffect(() => {
    if (completed) {
      setSelection(null);
      refreshRef.current?.focus();
    }
  }, [completed]);
  return {
    ...access,
    pending,
    refreshRef,
    confirmRef,
    queryError: access.query.isError ? errorMessage(access.query.error) : null,
    uninstallError: access.uninstall.isError && access.uninstall.variables?.scope === access.scope ? errorMessage(access.uninstall.error) : null,
    requestUninstall: (extension: InstalledExtensionSummary, trigger: HTMLButtonElement) => {
      access.uninstall.reset();
      triggerRef.current = trigger;
      setSelection({ scope: access.scope, extension });
    },
    cancel: () => {
      restoreFocus.current = true;
      setSelection(null);
    },
    confirm: () => {
      if (pending && access.runtimeAvailable && !access.uninstall.isPending) {
        access.uninstall.mutate(access.uninstallTarget(pending));
      }
    },
  };
}
