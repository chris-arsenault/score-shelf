import { useCallback, useEffect, useState } from "react";

/** Busy/error tracking for async actions (ADR 015). */
export function useAsyncAction() {
  const [busy, setBusy] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  const run = useCallback(async (label: string, fn: () => Promise<unknown>) => {
    setBusy(label);
    setError(null);
    try {
      await fn();
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setBusy(null);
    }
  }, []);

  const clearError = useCallback(() => setError(null), []);

  return { busy, error, run, clearError };
}

/** Loads data once per key and on demand, with shared busy/error state. */
export function useLoaded<T>(key: string, load: () => Promise<T>) {
  const [data, setData] = useState<T | null>(null);
  const action = useAsyncAction();
  const { run } = action;

  const reload = useCallback(
    () =>
      run("Loading", async () => {
        setData(await load());
      }),
    // `load` is recreated each render; `key` identifies what it loads.
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [key, run]
  );

  useEffect(() => {
    void reload();
  }, [reload]);

  return { data, reload, ...action };
}

export type Route = { view: "pieces" } | { view: "piece"; slug: string };

export function parseRoute(hash: string): Route {
  const match = /^#\/pieces\/([a-z0-9][a-z0-9_-]*)$/.exec(hash);
  return match ? { view: "piece", slug: match[1] } : { view: "pieces" };
}

/** Hash routing keeps the phone's back button working without a router. */
export function useHashRoute(): Route {
  const [route, setRoute] = useState<Route>(() => parseRoute(window.location.hash));
  useEffect(() => {
    const onChange = () => setRoute(parseRoute(window.location.hash));
    window.addEventListener("hashchange", onChange);
    return () => window.removeEventListener("hashchange", onChange);
  }, []);
  return route;
}
