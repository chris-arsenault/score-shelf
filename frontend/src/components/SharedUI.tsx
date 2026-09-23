import type { ReactNode } from "react";

export function ErrorBanner({ error, onDismiss }: { error: string | null; onDismiss: () => void }) {
  if (!error) return null;
  return (
    <div className="error-banner" role="alert">
      <span>{error}</span>
      <button type="button" className="btn btn-quiet" onClick={onDismiss}>
        Dismiss
      </button>
    </div>
  );
}

export function ViewHeader({ title, description }: { title: string; description: ReactNode }) {
  return (
    <header className="view-header">
      <h1>{title}</h1>
      <div className="view-desc">{description}</div>
    </header>
  );
}
