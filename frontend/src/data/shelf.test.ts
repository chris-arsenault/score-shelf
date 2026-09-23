import { afterEach, describe, expect, it, vi } from "vitest";

vi.mock("../auth", () => ({ getAccessToken: vi.fn(async () => "token-123") }));

import { ApiError, resetFetchForTests, setFetchForTests } from "./api";
import { contentTypeOf, kindOf, listPieces, uploadVersion } from "./shelf";

type Call = { url: string; init: RequestInit | undefined };

function fakeFetch(responses: Array<{ status: number; body?: unknown }>) {
  const calls: Call[] = [];
  setFetchForTests(async (input, init) => {
    calls.push({ url: String(input), init });
    const next = responses.shift() ?? { status: 200, body: {} };
    const text = next.body === undefined ? "" : JSON.stringify(next.body);
    return new Response(text, { status: next.status });
  });
  return calls;
}

afterEach(() => resetFetchForTests());

describe("api requests", () => {
  it("sends the access token and returns the parsed body", async () => {
    const calls = fakeFetch([{ status: 200, body: { pieces: [{ slug: "a" }] } }]);
    const pieces = await listPieces();
    expect(pieces).toEqual([{ slug: "a" }]);
    const headers = calls[0].init?.headers as Record<string, string>;
    expect(headers.Authorization).toBe("Bearer token-123");
    expect(calls[0].url.endsWith("/pieces")).toBe(true);
  });

  it("turns error responses into ApiError with the server message", async () => {
    fakeFetch([{ status: 400, body: { code: "validation_error", message: "label is required" } }]);
    await expect(listPieces()).rejects.toEqual(
      new ApiError(400, "validation_error", "label is required")
    );
  });
});

describe("uploadVersion", () => {
  it("creates the version, uploads each file to its presigned URL, then commits", async () => {
    const calls = fakeFetch([
      {
        status: 201,
        body: {
          version_id: "v-1",
          number: 4,
          uploads: [
            {
              file_id: "f-1",
              filename: "edit.musicxml",
              upload: { url: "https://s3.test/key", method: "PUT", headers: { a: "b" } },
            },
          ],
        },
      },
      { status: 200 },
      { status: 200, body: {} },
    ]);
    const file = new File(["<score/>"], "edit.musicxml");
    const number = await uploadVersion("boreal_pocket", "bar 12 fix", [file]);
    expect(number).toBe(4);
    const created = JSON.parse(String(calls[0].init?.body));
    expect(created.files[0]).toMatchObject({ filename: "edit.musicxml", kind: "musicxml" });
    expect(calls[1].url).toBe("https://s3.test/key");
    expect(calls[1].init?.method).toBe("PUT");
    expect(calls[2].url.endsWith("/versions/v-1/commit")).toBe(true);
  });

  it("stops before committing when the upload is rejected", async () => {
    const calls = fakeFetch([
      {
        status: 201,
        body: {
          version_id: "v-1",
          number: 1,
          uploads: [
            { file_id: "f", filename: "a.mid", upload: { url: "u", method: "PUT", headers: {} } },
          ],
        },
      },
      { status: 403 },
    ]);
    await expect(uploadVersion("p", "x", [new File(["m"], "a.mid")])).rejects.toBeInstanceOf(
      ApiError
    );
    expect(calls).toHaveLength(2);
  });
});

describe("file kinds", () => {
  it("maps extensions to kinds and content types", () => {
    expect(kindOf("score.MusicXML")).toBe("musicxml");
    expect(kindOf("score.mid")).toBe("midi");
    expect(kindOf("notes.txt")).toBe("other");
    expect(contentTypeOf(new File(["x"], "a.mid"))).toBe("audio/midi");
    expect(contentTypeOf(new File(["x"], "a.bin", { type: "application/x-test" }))).toBe(
      "application/x-test"
    );
  });
});
