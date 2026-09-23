import { useCallback, useEffect, useState } from "react";

import { getSession, signOut } from "./auth";
import { useHashRoute } from "./hooks";
import { PieceView } from "./views/PieceView";
import { PiecesView } from "./views/PiecesView";
import { SignInView } from "./views/SignInView";

type SessionState = "checking" | "signedIn" | "signedOut";

export function App() {
  const [session, setSession] = useState<SessionState>("checking");
  const route = useHashRoute();

  const refreshSession = useCallback(() => {
    getSession()
      .then((current) => setSession(current ? "signedIn" : "signedOut"))
      .catch(() => setSession("signedOut"));
  }, []);

  useEffect(refreshSession, [refreshSession]);

  if (session === "checking") return null;
  if (session === "signedOut") return <SignInView onSignedIn={refreshSession} />;

  const handleSignOut = () => {
    signOut();
    setSession("signedOut");
  };

  return (
    <>
      <nav className="topbar">
        <a href="#/">Score Shelf</a>
        <button type="button" className="btn btn-quiet" onClick={handleSignOut}>
          Sign out
        </button>
      </nav>
      {route.view === "piece" ? <PieceView slug={route.slug} /> : <PiecesView />}
    </>
  );
}
