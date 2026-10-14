import { useEffect, useId, useRef } from "react";

import type { ShelfFile } from "../data/shelf";
import { downloadUrl } from "../data/shelf";
import { useAsyncAction } from "../hooks";
import { useScoreRenderer } from "../score/useScoreRenderer";
import "./ScorePreview.css";

type Props = { file: ShelfFile; onClose: () => void };

export function ScorePreview({ file, onClose }: Props) {
  const dialog = useRef<HTMLDialogElement>(null);
  const container = useRef<HTMLDivElement>(null);
  const titleId = useId();
  const score = useScoreRenderer(file.id, container);
  const download = useAsyncAction();

  useEffect(() => {
    dialog.current?.showModal();
    const overflow = document.body.style.overflow;
    document.body.style.overflow = "hidden";
    return () => {
      document.body.style.overflow = overflow;
    };
  }, []);

  const close = () => dialog.current?.close();
  const openDownload = () =>
    download.run("Preparing download", async () => {
      window.location.assign(await downloadUrl(file.id));
    });

  return (
    <dialog ref={dialog} className="score-preview" aria-labelledby={titleId} onClose={onClose}>
      <header className="score-preview-header">
        <h2 id={titleId}>{file.filename}</h2>
        <button type="button" className="btn btn-quiet" onClick={close}>
          Close
        </button>
      </header>
      <div className="score-preview-toolbar">
        <ZoomControls score={score} />
        <button
          type="button"
          className="btn btn-quiet"
          disabled={!!download.busy}
          onClick={openDownload}
        >
          Download
        </button>
      </div>
      <div className="score-preview-status" role="status">
        {(score.busy || download.busy) && `${score.busy || download.busy}…`}
        {(download.error || score.error) && (
          <span role="alert">{download.error || score.error}</span>
        )}
      </div>
      {/* eslint-disable-next-line jsx-a11y/no-noninteractive-tabindex -- Keyboard users need to focus the scrollable score for arrow-key navigation. */}
      <div className="score-preview-scroll" role="region" aria-label="Sheet music" tabIndex={0}>
        <div ref={container} className="score-preview-paper" />
      </div>
    </dialog>
  );
}

function ZoomControls({ score }: { score: ReturnType<typeof useScoreRenderer> }) {
  const disabled = !score.ready || !!score.busy;
  return (
    <div className="score-preview-zoom" role="group" aria-label="Score zoom">
      <button
        type="button"
        className="btn btn-quiet"
        aria-label="Zoom out"
        disabled={disabled || score.zoom <= 0.5}
        onClick={() => score.setZoom((value) => Math.max(0.5, value - 0.25))}
      >
        −
      </button>
      <output aria-label="Zoom level">{Math.round(score.zoom * 100)}%</output>
      <button
        type="button"
        className="btn btn-quiet"
        aria-label="Zoom in"
        disabled={disabled || score.zoom >= 2}
        onClick={() => score.setZoom((value) => Math.min(2, value + 0.25))}
      >
        +
      </button>
      <button
        type="button"
        className="btn btn-quiet"
        disabled={disabled || score.zoom === 1}
        onClick={() => score.setZoom(1)}
      >
        Reset zoom
      </button>
    </div>
  );
}
