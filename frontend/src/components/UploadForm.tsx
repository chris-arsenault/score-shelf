import { useState } from "react";
import type { FormEvent } from "react";

import { uploadVersion } from "../data/shelf";
import { useAsyncAction } from "../hooks";
import { ErrorBanner } from "./SharedUI";

type Props = { slug: string; onUploaded: () => void };

/** Uploads hand-edited files as a new owner version of the piece. */
export function UploadForm({ slug, onUploaded }: Props) {
  const { busy, error, run, clearError } = useAsyncAction();
  const [label, setLabel] = useState("");
  const [files, setFiles] = useState<File[]>([]);
  const [done, setDone] = useState<number | null>(null);

  const submit = (event: FormEvent) => {
    event.preventDefault();
    void run("Uploading", async () => {
      const number = await uploadVersion(slug, label || "hand edit", files);
      setDone(number);
      setFiles([]);
      setLabel("");
      onUploaded();
    });
  };

  return (
    <form className="panel stack" onSubmit={submit}>
      <h2>Upload a hand edit</h2>
      <ErrorBanner error={error} onDismiss={clearError} />
      {done !== null && <p className="notice">Saved as version {done}.</p>}
      <label className="field">
        Label
        <input placeholder="hand edit" value={label} onChange={(e) => setLabel(e.target.value)} />
      </label>
      <label className="field">
        Files
        <input
          type="file"
          multiple
          accept=".musicxml,.xml,.mxl,.mid,.midi,.pdf"
          onChange={(e) => setFiles(Array.from(e.target.files ?? []))}
        />
      </label>
      <button type="submit" className="btn" disabled={busy !== null || files.length === 0}>
        {busy ?? "Upload"}
      </button>
    </form>
  );
}
