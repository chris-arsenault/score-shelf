// @vitest-environment happy-dom
import { act, StrictMode } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { ScorePreview } from "./ScorePreview";
import { loadScoreFile, downloadUrl } from "../data/shelf";
import { loadScoreRenderer } from "../score/renderer";

vi.mock("../data/shelf", () => ({ loadScoreFile: vi.fn(), downloadUrl: vi.fn() }));
vi.mock("../score/renderer", () => ({ loadScoreRenderer: vi.fn() }));

const file = {
  id: "score-id",
  filename: "score.musicxml",
  kind: "musicxml" as const,
  content_type: "application/xml",
  size_bytes: 100,
};

let root: Root;
let host: HTMLDivElement;
const render = vi.fn();
const dispose = vi.fn();

beforeEach(() => {
  vi.stubGlobal("IS_REACT_ACT_ENVIRONMENT", true);
  vi.mocked(loadScoreFile).mockResolvedValue(new Blob(["<score-partwise/>"]));
  vi.mocked(loadScoreRenderer).mockImplementation(async (_target, _file, signal) => {
    signal.addEventListener("abort", dispose, { once: true });
    return { render, dispose };
  });
  vi.spyOn(HTMLDialogElement.prototype, "showModal").mockImplementation(function () {
    this.setAttribute("open", "");
  });
  host = document.createElement("div");
  document.body.append(host);
  root = createRoot(host);
});

afterEach(async () => {
  await act(async () => root.unmount());
  host.remove();
  vi.restoreAllMocks();
  vi.clearAllMocks();
  vi.unstubAllGlobals();
});

async function mount() {
  await act(async () => {
    root.render(
      <StrictMode>
        <ScorePreview file={file} onClose={vi.fn()} />
      </StrictMode>
    );
  });
}

async function rendered() {
  await act(async () => {
    await vi.waitFor(() => expect(render).toHaveBeenCalled());
  });
}

function button(label: string) {
  const element = host.querySelector<HTMLButtonElement>(`[aria-label="${label}"]`);
  if (!element) throw new Error(`Missing button: ${label}`);
  return element;
}

describe("score preview", () => {
  it("keeps the active loading status when Strict Mode abandons an earlier load", async () => {
    let finishFirst = (_file: Blob) => {};
    const abandoned = new Promise<Blob>((resolve) => {
      finishFirst = resolve;
    });
    const pending = new Promise<Blob>(() => {});
    vi.mocked(loadScoreFile).mockReturnValueOnce(abandoned).mockReturnValueOnce(pending);
    await mount();
    await act(async () => finishFirst(new Blob(["<score-partwise/>"])));
    expect(host.querySelector('[role="status"]')?.textContent).toContain("Loading score");
    expect(button("Zoom in").disabled).toBe(true);
    expect(loadScoreRenderer).not.toHaveBeenCalled();
  });

  it("renders at normal scale, zooms, and aborts the file load and renderer on close", async () => {
    await mount();
    await rendered();
    expect(render).toHaveBeenLastCalledWith(1);
    expect(host.querySelector("dialog")?.open).toBe(true);
    expect(document.body.style.overflow).toBe("hidden");
    await act(async () => button("Zoom in").click());
    await act(async () => {
      await vi.waitFor(() => expect(render).toHaveBeenLastCalledWith(1.25));
    });
    expect(host.querySelector("output")?.textContent).toBe("125%");
    const signal = vi.mocked(loadScoreFile).mock.calls.at(-1)?.[1];
    await act(async () => root.unmount());
    expect(signal?.aborted).toBe(true);
    expect(dispose).toHaveBeenCalled();
    expect(document.body.style.overflow).toBe("");
    root = createRoot(host);
  });

  it("shows download failures in the viewer and leaves zoom disabled", async () => {
    vi.mocked(loadScoreFile).mockRejectedValue(new Error("The score could not be downloaded."));
    await mount();
    expect(host.querySelector('[role="alert"]')?.textContent).toContain("could not be downloaded");
    expect(button("Zoom in").disabled).toBe(true);
    expect(loadScoreRenderer).not.toHaveBeenCalled();
  });

  it("keeps the download action available when engraving fails", async () => {
    render.mockImplementationOnce(() => {
      throw new Error("This file could not be rendered as MusicXML.");
    });
    await mount();
    await rendered();
    expect(host.querySelector('[role="alert"]')?.textContent).toContain("could not be rendered");
    const download = Array.from(host.querySelectorAll("button")).find(
      (element) => element.textContent === "Download"
    );
    vi.mocked(downloadUrl).mockRejectedValueOnce(new Error("Download unavailable."));
    await act(async () => download?.click());
    expect(downloadUrl).toHaveBeenCalledWith(file.id);
    expect(host.querySelector('[role="alert"]')?.textContent).toBe("Download unavailable.");
  });
});
