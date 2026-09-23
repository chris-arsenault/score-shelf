/** Typed calls to the Score Shelf API. */

import { apiGet, apiPost, putPresigned } from "./api";
import type { PresignedUpload } from "./api";

export type FileKind = "musicxml" | "midi" | "audio" | "pdf" | "other";

export type ShelfFile = {
  id: string;
  kind: FileKind;
  filename: string;
  content_type: string;
  size_bytes: number;
};

export type Version = {
  id: string;
  number: number;
  label: string;
  notes: string;
  source: "agent" | "owner";
  source_ref: string | null;
  created_at: string;
  files: ShelfFile[];
};

export type PieceSummary = {
  slug: string;
  title: string;
  version_count: number;
  latest: Version | null;
};

export type PieceDetail = {
  slug: string;
  title: string;
  versions: Version[];
};

type CreatedVersion = {
  version_id: string;
  number: number;
  uploads: { file_id: string; filename: string; upload: PresignedUpload }[];
};

export async function listPieces(): Promise<PieceSummary[]> {
  const body = await apiGet<{ pieces: PieceSummary[] }>("/pieces");
  return body.pieces;
}

export function getPiece(slug: string): Promise<PieceDetail> {
  return apiGet<PieceDetail>(`/pieces/${encodeURIComponent(slug)}`);
}

export async function downloadUrl(fileId: string): Promise<string> {
  const body = await apiGet<{ url: string }>(`/files/${encodeURIComponent(fileId)}/download`);
  return body.url;
}

const KIND_BY_EXTENSION: Record<string, FileKind> = {
  musicxml: "musicxml",
  xml: "musicxml",
  mxl: "musicxml",
  mid: "midi",
  midi: "midi",
  wav: "audio",
  mp3: "audio",
  flac: "audio",
  pdf: "pdf",
};

export function kindOf(filename: string): FileKind {
  const extension = filename.split(".").pop()?.toLowerCase() ?? "";
  return KIND_BY_EXTENSION[extension] ?? "other";
}

export function contentTypeOf(file: File): string {
  if (file.type) return file.type;
  const kind = kindOf(file.name);
  if (kind === "musicxml") return "application/vnd.recordare.musicxml+xml";
  if (kind === "midi") return "audio/midi";
  return "application/octet-stream";
}

/** Creates an owner version, uploads each file, then commits it. */
export async function uploadVersion(slug: string, label: string, files: File[]): Promise<number> {
  const created = await apiPost<CreatedVersion>(`/pieces/${encodeURIComponent(slug)}/versions`, {
    label,
    files: files.map((file) => ({
      filename: file.name,
      kind: kindOf(file.name),
      content_type: contentTypeOf(file),
      size_bytes: file.size,
    })),
  });
  for (const upload of created.uploads) {
    const file = files.find((candidate) => candidate.name === upload.filename);
    if (!file) throw new Error(`No file named ${upload.filename}`);
    await putPresigned(upload.upload, file);
  }
  await apiPost(`/versions/${created.version_id}/commit`);
  return created.number;
}
