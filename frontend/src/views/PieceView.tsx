import { ErrorBanner, ViewHeader } from "../components/SharedUI";
import { UploadForm } from "../components/UploadForm";
import { formatBytes, formatDate, sourceLabel } from "../data/format";
import { downloadUrl, getPiece } from "../data/shelf";
import type { ShelfFile, Version } from "../data/shelf";
import { useAsyncAction, useLoaded } from "../hooks";

type Props = { slug: string };

export function PieceView({ slug }: Props) {
  const piece = useLoaded(`piece:${slug}`, () => getPiece(slug));
  const download = useAsyncAction();

  const open = (file: ShelfFile) =>
    download.run(`Preparing ${file.filename}`, async () => {
      window.location.assign(await downloadUrl(file.id));
    });

  return (
    <main className="page">
      <a className="back" href="#/">
        ← All pieces
      </a>
      <ViewHeader
        title={piece.data?.title ?? slug}
        description={piece.data ? `${piece.data.versions.length} versions` : "Loading…"}
      />
      <ErrorBanner error={piece.error} onDismiss={piece.clearError} />
      <ErrorBanner error={download.error} onDismiss={download.clearError} />
      {download.busy && <p className="notice">{download.busy}…</p>}
      <ol className="version-list">
        {(piece.data?.versions ?? []).map((version) => (
          <VersionItem key={version.id} version={version} onOpen={open} />
        ))}
      </ol>
      <UploadForm slug={slug} onUploaded={piece.reload} />
    </main>
  );
}

function VersionItem({ version, onOpen }: { version: Version; onOpen: (f: ShelfFile) => void }) {
  return (
    <li className="panel">
      <div className="version-head">
        <span className="version-number">v{version.number}</span>
        <span className="version-label">{version.label}</span>
      </div>
      <div className="card-sub">
        {sourceLabel(version.source)} · {formatDate(version.created_at)}
        {version.source_ref && ` · ${version.source_ref}`}
      </div>
      {version.notes && <p className="version-notes">{version.notes}</p>}
      <ul className="file-list">
        {version.files.map((file) => (
          <li key={file.id}>
            <button type="button" className="btn btn-file" onClick={() => onOpen(file)}>
              <span>{file.filename}</span>
              <span className="card-meta">{formatBytes(file.size_bytes)}</span>
            </button>
          </li>
        ))}
      </ul>
    </li>
  );
}
