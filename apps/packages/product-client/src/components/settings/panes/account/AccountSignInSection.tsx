import { Button } from "#product/primitives/Button";
import { SettingsSection } from "#product/primitives/patterns/settings/SettingsSection";
import { PasswordSignInForm } from "#product/components/auth/PasswordSignInForm";
import { useGitHubSignIn } from "#product/hooks/auth/workflows/use-github-sign-in";
import { usePasswordSignIn } from "#product/hooks/auth/workflows/use-password-sign-in";
import { useControlPlaneHealth } from "#product/hooks/access/cloud/use-control-plane-health";
import { useDesktopAuthMethods } from "#product/hooks/access/cloud/auth/use-auth-methods";
import { useGitHubDesktopAuthAvailability } from "#product/hooks/access/cloud/auth/use-github-auth-availability";

export function AccountSignInSection() {
  const github = useGitHubSignIn();
  const password = usePasswordSignIn();
  const health = useControlPlaneHealth();
  const methods = useDesktopAuthMethods();
  const availability = useGitHubDesktopAuthAvailability();
  const busy = github.submitting || password.submitting;
  const checking = health.isFetching || github.signInChecking || password.signInChecking;
  const available = health.data === true && (github.signInAvailable || password.signInAvailable);
  async function retry() {
    github.clearError();
    password.clearError();
    const result = await health.refetch();
    if (result.data === true) await Promise.allSettled([methods.refetch(), availability.refetch()]);
  }
  return <SettingsSection title="Sign in to Proliferate"
    description="Sign in for account sync and managed services. Local agents remain available without an account.">
    <div className="space-y-3">
      {health.data === true && github.signInAvailable ? <Button type="button" variant="secondary" loading={github.submitting} disabled={busy}
        onClick={() => { void github.signIn().catch(() => {}); }}>Continue with GitHub</Button> : null}
      {health.data === true && password.signInAvailable ? <PasswordSignInForm submitting={password.submitting} disabled={busy}
        onSubmit={(email, secret) => { void password.signIn(email, secret).catch(() => {}); }} /> : null}
      {!available && checking ? <p role="status" className="text-ui-sm text-muted-foreground">Checking account sign-in…</p> : null}
      {!available && !checking ? <p role="status" className="text-ui-sm text-muted-foreground">Account sign-in is unavailable from the selected server. Retry or change the account server below. Local agents remain available.</p> : null}
      {github.error || password.error ? <p role="alert" className="text-ui-sm text-destructive">{github.error ?? password.error}</p> : null}
      <Button type="button" variant="ghost" size="sm" disabled={busy || checking} onClick={() => { void retry(); }}>Retry account connection</Button>
    </div>
  </SettingsSection>;
}
