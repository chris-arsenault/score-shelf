import { useEffect, useState } from "react";
import type { RefObject } from "react";

import { loadScoreFile } from "../data/shelf";
import { useAsyncAction } from "../hooks";
import { loadScoreRenderer } from "./renderer";
import type { ScoreRenderer } from "./renderer";

export function useScoreRenderer(fileId: string, container: RefObject<HTMLDivElement | null>) {
  const [renderer, setRenderer] = useState<ScoreRenderer | null>(null);
  const [zoom, setZoom] = useState(1);
  const action = useAsyncAction();
  const { run } = action;

  useEffect(() => {
    const target = container.current;
    if (!target) return;
    const controller = new AbortController();
    void run("Loading score", async () => {
      try {
        const file = await loadScoreFile(fileId, controller.signal);
        controller.signal.throwIfAborted();
        const score = await loadScoreRenderer(target, file, controller.signal);
        if (!controller.signal.aborted) setRenderer(score);
      } catch (error) {
        if (!controller.signal.aborted) throw error;
      }
    });
    return () => controller.abort();
  }, [fileId, container, run]);

  useEffect(() => {
    const target = container.current;
    if (!renderer || !target) return;
    let frame = 0;
    const draw = async () => renderer.render(zoom);
    const drawFrame = () => void run("Rendering score", draw);
    const render = () => {
      cancelAnimationFrame(frame);
      frame = requestAnimationFrame(drawFrame);
    };
    let width = target.clientWidth;
    const observer = new ResizeObserver(() => {
      if (target.clientWidth === width) return;
      width = target.clientWidth;
      render();
    });
    observer.observe(target);
    render();
    return () => {
      cancelAnimationFrame(frame);
      observer.disconnect();
    };
  }, [renderer, container, zoom, run]);

  return { ...action, zoom, setZoom, ready: renderer !== null };
}
