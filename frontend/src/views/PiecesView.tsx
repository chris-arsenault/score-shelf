import { ErrorBanner, ViewHeader } from "../components/SharedUI";
import { formatDate, sourceLabel } from "../data/format";
import { listPieces } from "../data/shelf";
import { useLoaded } from "../hooks";

export function PiecesView() {
  const { data: pieces, busy, error, clearError, reload } = useLoaded("pieces", listPieces);

  return (
    <main className="page">
      <ViewHeader title="Score Shelf" description="Every published piece, newest first." />
      <ErrorBanner error={error} onDismiss={clearError} />
      <div className="toolbar">
        <button type="button" className="btn btn-quiet" onClick={reload} disabled={busy !== null}>
          {busy ? "Loading…" : "Refresh"}
        </button>
      </div>
      {pieces && pieces.length === 0 && (
        <div className="empty-state">
          No pieces yet. Versions appear here once the agent publishes an export.
        </div>
      )}
      <ul className="card-list">
        {(pieces ?? []).map((piece) => (
          <li key={piece.slug}>
            <a className="card" href={`#/pieces/${piece.slug}`}>
              <span className="card-title">{piece.title}</span>
              {piece.latest && (
                <span className="card-sub">
                  v{piece.latest.number} · {piece.latest.label} · {sourceLabel(piece.latest.source)}{" "}
                  · {formatDate(piece.latest.created_at)}
                </span>
              )}
              <span className="card-meta">
                {piece.version_count} version{piece.version_count === 1 ? "" : "s"}
              </span>
            </a>
          </li>
        ))}
      </ul>
    </main>
  );
}
