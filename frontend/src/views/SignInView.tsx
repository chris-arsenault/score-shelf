import { useState } from "react";
import type { FormEvent } from "react";

import { signIn } from "../auth";
import type { SignInResult, SoftwareTokenMfaChallenge } from "../auth";
import { ErrorBanner, ViewHeader } from "../components/SharedUI";
import { useAsyncAction } from "../hooks";

type Props = { onSignedIn: () => void };
type Run = (label: string, fn: () => Promise<unknown>) => Promise<void>;
type FormProps = { busy: string | null; run: Run };

export function SignInView({ onSignedIn }: Props) {
  const { busy, error, run, clearError } = useAsyncAction();
  const [challenge, setChallenge] = useState<SoftwareTokenMfaChallenge | null>(null);
  const [enrollmentUrl, setEnrollmentUrl] = useState<string | null>(null);

  const handleResult = (result: SignInResult) => {
    if (result.status === "signedIn") onSignedIn();
    if (result.status === "softwareTokenMfaRequired") setChallenge(result.challenge);
    if (result.status === "mfaSetupRequired") setEnrollmentUrl(result.redirect.enrollmentUrl);
  };

  return (
    <main className="page narrow">
      <ViewHeader title="Score Shelf" description="Sign in to your scores." />
      <ErrorBanner error={error} onDismiss={clearError} />
      {enrollmentUrl && (
        <p className="notice">
          Set up an authenticator first at <a href={enrollmentUrl}>{enrollmentUrl}</a>, then sign in
          again.
        </p>
      )}
      {challenge ? (
        <CodeForm busy={busy} run={run} challenge={challenge} onSignedIn={onSignedIn} />
      ) : (
        <PasswordForm busy={busy} run={run} onResult={handleResult} />
      )}
    </main>
  );
}

function PasswordForm({
  busy,
  run,
  onResult,
}: FormProps & { onResult: (r: SignInResult) => void }) {
  const [username, setUsername] = useState("");
  const [password, setPassword] = useState("");

  const submit = (event: FormEvent) => {
    event.preventDefault();
    run("Signing in", async () => onResult(await signIn(username, password)));
  };

  return (
    <form className="stack" onSubmit={submit}>
      <label className="field">
        Username
        <input
          autoComplete="username"
          value={username}
          onChange={(e) => setUsername(e.target.value)}
        />
      </label>
      <label className="field">
        Password
        <input
          type="password"
          autoComplete="current-password"
          value={password}
          onChange={(e) => setPassword(e.target.value)}
        />
      </label>
      <button type="submit" className="btn" disabled={busy !== null}>
        {busy ?? "Sign in"}
      </button>
    </form>
  );
}

type CodeFormProps = FormProps & {
  challenge: SoftwareTokenMfaChallenge;
  onSignedIn: () => void;
};

function CodeForm({ busy, run, challenge, onSignedIn }: CodeFormProps) {
  const [code, setCode] = useState("");

  const submit = (event: FormEvent) => {
    event.preventDefault();
    run("Verifying", async () => {
      await challenge.submitCode(code);
      onSignedIn();
    });
  };

  return (
    <form className="stack" onSubmit={submit}>
      <label className="field">
        Authenticator code
        <input
          inputMode="numeric"
          autoComplete="one-time-code"
          value={code}
          onChange={(e) => setCode(e.target.value)}
        />
      </label>
      <button type="submit" className="btn" disabled={busy !== null}>
        {busy ?? "Verify"}
      </button>
    </form>
  );
}
