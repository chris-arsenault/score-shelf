export type ScoreRenderer = {
  render: (zoom: number) => void;
  dispose: () => void;
};

const RENDER_ERROR = "This file could not be rendered as MusicXML. You can still download it.";

/** Each instance owns its DOM so an abandoned load cannot clear a newer score. */
export async function loadScoreRenderer(
  container: HTMLElement,
  file: Blob,
  signal: AbortSignal
): Promise<ScoreRenderer> {
  const { OpenSheetMusicDisplay } = await import("opensheetmusicdisplay");
  signal.throwIfAborted();
  const paper = document.createElement("div");
  container.append(paper);
  const score = new OpenSheetMusicDisplay(paper, {
    backend: "svg",
    autoResize: false,
    pageFormat: "Endless",
    drawTitle: true,
  });
  score.setLogLevel("error");
  let disposed = false;
  const dispose = () => {
    if (disposed) return;
    disposed = true;
    signal.removeEventListener("abort", dispose);
    score.clear();
    paper.remove();
  };
  signal.addEventListener("abort", dispose, { once: true });
  try {
    await score.load(file);
    signal.throwIfAborted();
  } catch {
    dispose();
    signal.throwIfAborted();
    throw new Error(RENDER_ERROR);
  }
  return {
    render(zoom) {
      if (disposed) return;
      try {
        score.Zoom = zoom;
        score.render();
      } catch {
        score.clear();
        throw new Error(RENDER_ERROR);
      }
    },
    dispose,
  };
}
