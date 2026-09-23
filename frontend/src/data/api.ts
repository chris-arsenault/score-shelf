/**
 * The only module that talks HTTP (ADR 014). API calls carry the Cognito
 * access token; presigned S3 transfers carry nothing but their signed URL.
 */

import { getAccessToken } from "../auth";
import { config } from "../config";

export const API_BASE = config.apiBaseUrl.replace(/\/$/, "");

export class ApiError extends Error {
  constructor(
    readonly status: number,
    readonly code: string,
    message: string
  ) {
    super(message);
  }
}

type FetchLike = typeof fetch;
const browserFetch: FetchLike = (...args) => fetch(...args);
let fetchImpl: FetchLike = browserFetch;

/** Tests replace the transport; production uses the browser's fetch. */
export function setFetchForTests(impl: FetchLike): void {
  fetchImpl = impl;
}

export function resetFetchForTests(): void {
  fetchImpl = browserFetch;
}

async function request<T>(method: string, path: string, body?: unknown): Promise<T> {
  const token = await getAccessToken();
  const headers: Record<string, string> = {};
  if (token) headers["Authorization"] = `Bearer ${token}`;
  if (body !== undefined) headers["Content-Type"] = "application/json";
  const response = await fetchImpl(`${API_BASE}${path}`, {
    method,
    headers,
    body: body === undefined ? undefined : JSON.stringify(body),
  });
  const text = await response.text();
  const payload = text ? (JSON.parse(text) as Record<string, unknown>) : {};
  if (!response.ok) {
    const code = typeof payload.code === "string" ? payload.code : "api_error";
    const message = typeof payload.message === "string" ? payload.message : response.statusText;
    throw new ApiError(response.status, code, message);
  }
  return payload as T;
}

export function apiGet<T>(path: string): Promise<T> {
  return request<T>("GET", path);
}

export function apiPost<T>(path: string, body?: unknown): Promise<T> {
  return request<T>("POST", path, body);
}

export type PresignedUpload = {
  url: string;
  method: string;
  headers: Record<string, string>;
};

/** Sends file bytes straight to S3 through a presigned URL. */
export async function putPresigned(upload: PresignedUpload, file: Blob): Promise<void> {
  const response = await fetchImpl(upload.url, {
    method: upload.method,
    headers: upload.headers,
    body: file,
  });
  if (!response.ok) {
    throw new ApiError(response.status, "upload_failed", "The file upload was rejected.");
  }
}
